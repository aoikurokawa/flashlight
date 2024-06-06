use std::{borrow::Cow, sync::Arc, time::Duration};

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use async_utils::{retry_policy, spawn_retry_task};
use constants::{derive_perp_market_account, derive_spot_market_account, ProgramData};
use drift::{
    controller::position::PositionDirection,
    instructions::SpotFulfillmentType,
    state::{
        order_params::{ModifyOrderParams, OrderParams},
        perp_market::PerpMarket,
        spot_market::SpotMarket,
        state::State,
        user::{MarketType, Order, User},
    },
};
use fnv::FnvHashMap;
use futures_util::{future::BoxFuture, FutureExt, StreamExt};
use log::{debug, warn};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::RpcAccountInfoConfig,
};
use solana_sdk::{
    account::Account,
    address_lookup_table_account::AddressLookupTableAccount,
    clock::Slot,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::{v0, Message, VersionedMessage},
    pubkey::Pubkey,
    signature::{keypair_from_seed, Keypair},
    signer::Signer,
    transaction::VersionedTransaction,
};
use tokio::{
    select,
    sync::{
        watch::{self, Receiver},
        RwLock,
    },
};
use types::*;
use websocket_account_subscriber::WebsocketAccountSubscriber;

use crate::constants::state_account;

pub mod accounts;
pub mod addresses;
pub mod async_utils;
pub mod blockhash_subscriber;
pub mod config;
pub mod constants;
pub mod dlob;
pub mod drift_client;
pub mod drift_client_config;
pub mod event_emitter;
pub mod events;
pub mod jupiter;
pub mod marketmap;
pub mod math;
pub mod memcmp;
pub mod oraclemap;
pub mod priority_fee;
pub mod slot_subscriber;
pub mod tx;
pub mod types;
pub mod user;
pub mod user_config;
pub mod user_stats;
pub mod user_stats_config;
pub mod usermap;
pub mod utils;
pub mod websocket_account_subscriber;
pub mod websocket_program_account_subscriber;

type AccountCache = Arc<RwLock<FnvHashMap<Pubkey, Receiver<(Account, Slot)>>>>;

/// Provides solana Account fetching API
pub trait AccountProvider: 'static + Sized + Send + Sync {
    // TODO: async fn when it stabilizes
    /// Return the Account information of `account`
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>>;
    /// the HTTP endpoint URL
    fn endpoint(&self) -> String;
    /// return configured commitment level of the provider
    fn commitment_config(&self) -> CommitmentConfig;
}

/// Account provider that always fetches from RPC
#[derive(Clone)]
pub struct RpcAccountProvider {
    client: Arc<RpcClient>,
}

impl RpcAccountProvider {
    pub fn new(endpoint: &str) -> Self {
        Self::with_commitment(endpoint, CommitmentConfig::confirmed())
    }
    /// Create a new RPC account provider with provided commitment level
    pub fn with_commitment(endpoint: &str, commitment: CommitmentConfig) -> Self {
        Self {
            client: Arc::new(RpcClient::new_with_commitment(
                endpoint.to_string(),
                commitment,
            )),
        }
    }
    async fn get_account_impl(&self, account: Pubkey) -> SdkResult<Account> {
        let account_data: Account = self.client.get_account(&account).await?;
        Ok(account_data)
    }
}

impl AccountProvider for RpcAccountProvider {
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>> {
        self.get_account_impl(account).boxed()
    }
    fn endpoint(&self) -> String {
        self.client.url()
    }
    fn commitment_config(&self) -> CommitmentConfig {
        self.client.commitment()
    }
}

/// Account provider using websocket subscriptions to receive and cache account updates
#[derive(Clone)]
pub struct WsAccountProvider {
    url: String,
    rpc_client: Arc<RpcClient>,
    /// map from account pubkey to (account data, last modified ts)
    account_cache: AccountCache,
}

struct AccountSubscription {
    account: Pubkey,
    url: String,
    rpc_client: Arc<RpcClient>,
    /// sink for account updates
    tx: Arc<watch::Sender<(Account, Slot)>>,
}

impl AccountSubscription {
    const RPC_CONFIG: RpcAccountInfoConfig = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64Zstd),
        data_slice: None,
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
        min_context_slot: None,
    };
    async fn stream_fn(self) {
        let ws_client =
            match PubsubClient::new(self.url.as_str().replace("http", "ws").as_str()).await {
                Ok(ws_client) => ws_client,
                Err(err) => {
                    warn!(target: "account", "connect client {:?} failed: {err:?}", self.account);
                    return;
                }
            };

        let result = ws_client
            .account_subscribe(&self.account, Some(Self::RPC_CONFIG))
            .await;

        if let Err(err) = result {
            warn!(target: "account", "subscribe account {:?} failed: {err:?}", self.account);
            return;
        }
        debug!(target: "account", "start account stream {:?}", self.account);
        let (mut account_stream, unsub_fn) = result.unwrap();

        let mut poll_interval = tokio::time::interval(Duration::from_secs(10));
        let _ = poll_interval.tick().await; // ignore, immediate first tick
        loop {
            select! {
                biased;
                response = account_stream.next() => {
                    if let Some(account_update) = response {
                        let slot = account_update.context.slot;
                        let account_data = account_update
                            .value
                            .decode::<Account>()
                            .expect("account");
                        self.tx.send_if_modified(|current| {
                            if slot > current.1 {
                                debug!(target: "account", "stream update writing to cache");
                                *current = (account_data, slot);
                                true
                            } else {
                                debug!(target: "account", "stream update old");
                               false
                            }
                        });
                    } else {
                        // websocket subscription/stream closed, try reconnect..
                        warn!(target: "account", "account stream closed: {:?}", self.account);
                        break;
                    }
                }
                _ = poll_interval.tick() => {
                    if let Ok(account_data) = self.rpc_client.get_account_with_config(&self.account, Default::default()).await {
                        self.tx.send_if_modified(|current| {
                            let slot = account_data.context.slot;
                            // only update with polled value if its newer
                            if slot > current.1 {
                                debug!(target: "account", "poll update, writing to cache");
                                *current = (account_data.value.unwrap(), slot);
                                true
                            } else {
                                debug!(target: "account", "poll update, too old");
                                false
                            }
                        });
                    } else {
                        // consecutive errors would indicate an issue, there's not much that can be done besides log/panic...
                    }
                }
            }
        }
        unsub_fn().await;
        warn!(target: "account", "stream ended: {:?}", self.account);
    }
}

impl WsAccountProvider {
    /// Create a new WsAccountProvider given an endpoint that serves both http(s) and ws(s)
    pub async fn new(url: &str) -> SdkResult<Self> {
        Self::new_with_commitment(url, CommitmentConfig::confirmed()).await
    }
    /// Create a new WsAccountProvider with provided commitment level
    pub async fn new_with_commitment(url: &str, commitment: CommitmentConfig) -> SdkResult<Self> {
        Ok(Self {
            url: url.to_string(),
            rpc_client: Arc::new(RpcClient::new_with_commitment(url.to_string(), commitment)),
            account_cache: Default::default(),
        })
    }
    /// Subscribe to account updates via web-socket and polling
    fn subscribe_account(&self, account: Pubkey, tx: watch::Sender<(Account, Slot)>) {
        let rpc_client = Arc::clone(&self.rpc_client);
        let tx = Arc::new(tx);
        let url = self.url.clone();
        spawn_retry_task(
            move || {
                let account_sub = AccountSubscription {
                    account,
                    url: url.clone(),
                    rpc_client: Arc::clone(&rpc_client),
                    tx: Arc::clone(&tx),
                };
                account_sub.stream_fn()
            },
            retry_policy::forever(5),
        );
    }
    /// Fetch an account and initiate subscription for future updates
    async fn get_account_impl(&self, account: Pubkey) -> SdkResult<Account> {
        {
            let cache = self.account_cache.read().await;
            if let Some(account_data_rx) = cache.get(&account) {
                let (account_data, _last_modified) = account_data_rx.borrow().clone();
                return Ok(account_data);
            }
        }

        // fetch initial account data, stream only updates on changes
        let account_data: Account = self.rpc_client.get_account(&account).await?;
        let (tx, rx) = watch::channel((account_data.clone(), 0));
        {
            let mut cache = self.account_cache.write().await;
            cache.insert(account, rx);
        }
        self.subscribe_account(account, tx);

        Ok(account_data)
    }
}

impl AccountProvider for WsAccountProvider {
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>> {
        self.get_account_impl(account).boxed()
    }
    fn endpoint(&self) -> String {
        self.rpc_client.url()
    }
    fn commitment_config(&self) -> CommitmentConfig {
        self.rpc_client.commitment()
    }
}

/// Composable Tx builder for Drift program
///
/// Prefer `DriftClient::init_tx`
///
/// ```ignore
/// use drift_sdk::{types::Context, TransactionBuilder, Wallet};
///
/// let wallet = Wallet::from_seed_bs58(Context::Dev, "seed");
/// let client = DriftClient::new("api.example.com").await.unwrap();
/// let account_data = client.get_account(wallet.default_sub_account()).await.unwrap();
///
/// let tx = TransactionBuilder::new(client.program_data, wallet.default_sub_account(), account_data.into())
///     .cancel_all_orders()
///     .place_orders(&[
///         NewOrder::default().build(),
///         NewOrder::default().build(),
///     ])
///     .legacy()
///     .build();
///
/// let signature = client.sign_and_send(tx, &wallet).await?;
/// ```
///
pub struct TransactionBuilder<'a> {
    /// contextual on-chain program data
    program_data: &'a ProgramData,
    /// sub-account data
    account_data: Cow<'a, User>,
    /// the drift sub-account address
    sub_account: Pubkey,
    /// either account authority or account delegate
    authority: Pubkey,
    /// ordered list of instructions
    ixs: Vec<Instruction>,
    /// use legacy transaction mode
    legacy: bool,
    /// add additional lookup tables (v0 only)
    lookup_tables: Vec<AddressLookupTableAccount>,
}

impl<'a> TransactionBuilder<'a> {
    /// Initialize a new `TransactionBuilder` for default signer
    ///
    /// `program_data` program data from chain
    /// `sub_account` drift sub-account address
    /// `account_data` drift sub-account data
    /// `delegated` set true to build tx for delegated signing
    pub fn new<'b>(
        program_data: &'b ProgramData,
        sub_account: Pubkey,
        account_data: Cow<'b, User>,
        delegated: bool,
    ) -> Self
    where
        'b: 'a,
    {
        Self {
            authority: if delegated {
                account_data.delegate
            } else {
                account_data.authority
            },
            program_data,
            account_data,
            sub_account,
            ixs: Default::default(),
            lookup_tables: vec![program_data.lookup_table.clone()],
            legacy: false,
        }
    }
    /// Use legacy tx mode
    pub fn legacy(mut self) -> Self {
        self.legacy = true;
        self
    }
    /// Set the tx lookup tables
    pub fn lookup_tables(mut self, lookup_tables: &[AddressLookupTableAccount]) -> Self {
        self.lookup_tables = lookup_tables.to_vec();
        self.lookup_tables
            .push(self.program_data.lookup_table.clone());

        self
    }
    /// Set the priority fee of the tx
    ///
    /// `microlamports_per_cu` the price per unit of compute in µ-lamports
    pub fn with_priority_fee(mut self, microlamports_per_cu: u64, cu_limit: Option<u32>) -> Self {
        let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_price(microlamports_per_cu);
        self.ixs.insert(0, cu_limit_ix);
        if let Some(cu_limit) = cu_limit {
            let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
            self.ixs.insert(1, cu_price_ix);
        }

        self
    }

    /// Deposit collateral into account
    pub fn deposit(
        mut self,
        amount: u64,
        spot_market_index: u16,
        user_token_account: Pubkey,
        reduce_only: Option<bool>,
    ) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::Deposit {
                state: *state_account(),
                user: self.sub_account,
                user_stats: Wallet::derive_stats_account(&self.authority, &constants::PROGRAM_ID),
                authority: self.authority,
                spot_market_vault: constants::derive_spot_market_vault(spot_market_index),
                user_token_account,
                token_program: constants::TOKEN_PROGRAM_ID,
            },
            &[self.account_data.as_ref()],
            &[],
            &[MarketId::spot(spot_market_index)],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::Deposit {
                market_index: spot_market_index,
                amount,
                reduce_only: reduce_only.unwrap_or(false),
            }),
        };

        self.ixs.push(ix);

        self
    }

    pub fn withdraw(
        mut self,
        amount: u64,
        spot_market_index: u16,
        user_token_account: Pubkey,
        reduce_only: Option<bool>,
    ) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::Withdraw {
                state: *state_account(),
                user: self.sub_account,
                user_stats: Wallet::derive_stats_account(&self.authority, &constants::PROGRAM_ID),
                authority: self.authority,
                spot_market_vault: constants::derive_spot_market_vault(spot_market_index),
                user_token_account,
                drift_signer: constants::derive_drift_signer(),
                token_program: constants::TOKEN_PROGRAM_ID,
            },
            &[self.account_data.as_ref()],
            &[],
            &[MarketId::spot(spot_market_index)],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::Withdraw {
                market_index: spot_market_index,
                amount,
                reduce_only: reduce_only.unwrap_or(false),
            }),
        };

        self.ixs.push(ix);

        self
    }

    /// Place new orders for account
    pub fn place_orders(mut self, orders: Vec<OrderParams>) -> Self {
        let readable_accounts: Vec<MarketId> = orders
            .iter()
            .map(|o| (o.market_index, o.market_type).into())
            .collect();

        let accounts = build_accounts(
            self.program_data,
            drift::accounts::PlaceOrder {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
            },
            &[self.account_data.as_ref()],
            readable_accounts.as_ref(),
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::PlaceOrders { params: orders }),
        };

        self.ixs.push(ix);

        self
    }

    /// Cancel all orders for account
    pub fn cancel_all_orders(mut self) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::CancelOrder {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
            },
            &[self.account_data.as_ref()],
            &[],
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::CancelOrders {
                market_index: None,
                market_type: None,
                direction: None,
            }),
        };
        self.ixs.push(ix);

        self
    }

    /// Cancel account's orders matching some criteria
    ///
    /// `market` - tuple of market ID and type (spot or perp)
    ///
    /// `direction` - long or short
    pub fn cancel_orders(
        mut self,
        market: (u16, MarketType),
        direction: Option<PositionDirection>,
    ) -> Self {
        let (idx, kind) = market;
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::CancelOrder {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
            },
            &[self.account_data.as_ref()],
            &[(idx, kind).into()],
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::CancelOrders {
                market_index: Some(idx),
                market_type: Some(kind),
                direction,
            }),
        };
        self.ixs.push(ix);

        self
    }

    /// Cancel orders given ids
    pub fn cancel_orders_by_id(mut self, order_ids: Vec<u32>) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::CancelOrder {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
            },
            &[self.account_data.as_ref()],
            &[],
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::CancelOrdersByIds { order_ids }),
        };
        self.ixs.push(ix);

        self
    }

    /// Cancel orders by given _user_ ids
    pub fn cancel_orders_by_user_id(mut self, user_order_ids: Vec<u8>) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::CancelOrder {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
            },
            &[self.account_data.as_ref()],
            &[],
            &[],
        );

        for user_order_id in user_order_ids {
            let ix = Instruction {
                program_id: constants::PROGRAM_ID,
                accounts: accounts.clone(),
                data: InstructionData::data(&drift::instruction::CancelOrderByUserId {
                    user_order_id,
                }),
            };
            self.ixs.push(ix);
        }

        self
    }

    /// Modify existing order(s) by order id
    pub fn modify_orders(mut self, orders: &[(u32, ModifyOrderParams)]) -> Self {
        for (order_id, params) in orders {
            let accounts = build_accounts(
                self.program_data,
                drift::accounts::PlaceOrder {
                    state: *state_account(),
                    authority: self.authority,
                    user: self.sub_account,
                },
                &[self.account_data.as_ref()],
                &[],
                &[],
            );

            let ix = Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::ModifyOrder {
                    order_id: Some(*order_id),
                    modify_order_params: params.clone(),
                }),
            };
            self.ixs.push(ix);
        }

        self
    }

    /// Modify existing order(s) by user order id
    pub fn modify_orders_by_user_id(mut self, orders: &[(u8, ModifyOrderParams)]) -> Self {
        for (user_order_id, params) in orders {
            let accounts = build_accounts(
                self.program_data,
                drift::accounts::PlaceOrder {
                    state: *state_account(),
                    authority: self.authority,
                    user: self.sub_account,
                },
                &[self.account_data.as_ref()],
                &[],
                &[],
            );

            let ix = Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::ModifyOrderByUserId {
                    user_order_id: *user_order_id,
                    modify_order_params: params.clone(),
                }),
            };
            self.ixs.push(ix);
        }

        self
    }

    /// Add a place and make instruction
    ///
    /// `order` the order to place
    /// `taker_info` taker account address and data
    /// `taker_order_id` the id of the taker's order to match with
    /// `referrer` pukey of the taker's referrer account, if any
    /// `fulfilment_type` type of fill for spot orders, ignored for perp orders
    pub fn place_and_make(
        mut self,
        order: OrderParams,
        taker_info: &(Pubkey, User),
        taker_order_id: u32,
        referrer: Option<Pubkey>,
        fulfillment_type: Option<SpotFulfillmentType>,
    ) -> Self {
        let (taker, taker_account) = taker_info;
        let is_perp = order.market_type == MarketType::Perp;
        let perp_writable = [MarketId::perp(order.market_index)];
        let spot_writable = [MarketId::spot(order.market_index), MarketId::QUOTE_SPOT];
        let mut accounts = build_accounts(
            self.program_data,
            drift::accounts::PlaceAndMake {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
                user_stats: Wallet::derive_stats_account(&self.authority, &constants::PROGRAM_ID),
                taker: *taker,
                taker_stats: Wallet::derive_stats_account(taker, &constants::PROGRAM_ID),
            },
            &[self.account_data.as_ref(), &taker_account],
            &[],
            if is_perp {
                &perp_writable
            } else {
                &spot_writable
            },
        );

        if let Some(referrer) = referrer {
            accounts.push(AccountMeta::new(
                Wallet::derive_stats_account(&referrer, &constants::PROGRAM_ID),
                false,
            ));
            accounts.push(AccountMeta::new(referrer, false));
        }

        let ix = if order.market_type == MarketType::Perp {
            Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::PlaceAndMakePerpOrder {
                    params: order,
                    taker_order_id,
                }),
            }
        } else {
            Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::PlaceAndMakeSpotOrder {
                    params: order,
                    taker_order_id,
                    fulfillment_type,
                }),
            }
        };

        self.ixs.push(ix);
        self
    }

    /// Add a place and take instruction
    ///
    /// `order` the order to place
    ///
    /// `maker_info` pubkey of the maker/counterparty to take against and account data
    ///
    /// `referrer` pubkey of the maker's referrer account, if any
    ///
    /// `fulfilment_type` type of fill for spot orders, ignored for perp orders
    pub fn place_and_take(
        mut self,
        order: OrderParams,
        maker_info: Option<(Pubkey, User)>,
        referrer: Option<Pubkey>,
        fulfillment_type: Option<SpotFulfillmentType>,
    ) -> Self {
        let mut user_accounts = vec![self.account_data.as_ref()];
        if let Some((ref _maker_pubkey, ref maker)) = maker_info {
            user_accounts.push(maker);
        }

        let is_perp = order.market_type == MarketType::Perp;
        let perp_writable = [MarketId::perp(order.market_index)];
        let spot_writable = [MarketId::spot(order.market_index), MarketId::QUOTE_SPOT];

        let mut accounts = build_accounts(
            self.program_data,
            drift::accounts::PlaceAndTake {
                state: *state_account(),
                authority: self.authority,
                user: self.sub_account,
                user_stats: Wallet::derive_stats_account(&self.authority, &constants::PROGRAM_ID),
            },
            user_accounts.as_slice(),
            &[],
            if is_perp {
                &perp_writable
            } else {
                &spot_writable
            },
        );

        if referrer.is_some_and(|r| !maker_info.is_some_and(|(m, _)| m == r)) {
            let referrer = referrer.unwrap();
            accounts.push(AccountMeta::new(
                Wallet::derive_stats_account(&referrer, &constants::PROGRAM_ID),
                false,
            ));
            accounts.push(AccountMeta::new(referrer, false));
        }

        let ix = if is_perp {
            Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::PlaceAndTakePerpOrder {
                    params: order,
                    maker_order_id: None,
                }),
            }
        } else {
            Instruction {
                program_id: constants::PROGRAM_ID,
                accounts,
                data: InstructionData::data(&drift::instruction::PlaceAndTakeSpotOrder {
                    params: order,
                    maker_order_id: None,
                    fulfillment_type,
                }),
            }
        };

        self.ixs.push(ix);
        self
    }

    pub fn update_funding_rate(
        mut self,
        market_index: u16,
        perp_market_pubkey: &Pubkey,
        oracle: &Pubkey,
    ) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::UpdateFundingRate {
                state: *state_account(),
                perp_market: *perp_market_pubkey,
                oracle: *oracle,
            },
            &[],
            &[],
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::UpdateFundingRate { market_index }),
        };
        self.ixs.push(ix);

        self
    }

    pub fn get_trigger_order_ix(
        mut self,
        user_account_pubkey: Pubkey,
        user_account: User,
        order: Order,
        filler_pubkey: Option<Pubkey>,
    ) {
        let filler_pubkey = filler_pubkey.unwrap_or(user_account_pubkey);

        // let remaining_account_params =
    }

    /// Build the transaction message ready for signing and sending
    pub fn build(self) -> VersionedMessage {
        if self.legacy {
            let message = Message::new(self.ixs.as_ref(), Some(&self.authority));
            VersionedMessage::Legacy(message)
        } else {
            let message = v0::Message::try_compile(
                &self.authority,
                self.ixs.as_slice(),
                self.lookup_tables.as_slice(),
                Default::default(),
            )
            .expect("ok");
            VersionedMessage::V0(message)
        }
    }

    pub fn program_data(&self) -> &ProgramData {
        self.program_data
    }

    pub fn account_data(&self) -> &Cow<'_, User> {
        &self.account_data
    }
}

/// Builds a set of required accounts from a user's open positions and additional given accounts
///
/// `base_accounts` base anchor accounts
///
/// `user` Drift user account data
///
/// `markets_readable` IDs of markets to include as readable
///
/// `markets_writable` IDs of markets to include as writable (takes priority over readable)
///
/// # Panics
///  if the user has positions in an unknown market (i.e unsupported by the SDK)
pub fn build_accounts(
    program_data: &ProgramData,
    base_accounts: impl ToAccountMetas,
    users: &[&User],
    markets_readable: &[MarketId],
    markets_writable: &[MarketId],
) -> Vec<AccountMeta> {
    // the order of accounts returned must be instruction, oracles, spot, perps see (https://github.com/drift-labs/protocol-v2/blob/master/programs/drift/src/instructions/optional_accounts.rs#L28)
    let mut seen = [0_u64; 2]; // [spot, perp]
    let mut accounts = Vec::<RemainingAccount>::default();

    // add accounts to the ordered list
    let mut include_market = |market_index: u16, market_type: MarketType, writable: bool| {
        let index_bit = 1_u64 << market_index as u8;
        // always safe since market type is 0 or 1
        let seen_by_type = unsafe { seen.get_unchecked_mut(market_type as usize % 2) };
        if *seen_by_type & index_bit > 0 {
            return;
        }
        *seen_by_type |= index_bit;

        let (account, oracle) = match market_type {
            MarketType::Spot => {
                let SpotMarket { pubkey, oracle, .. } = program_data
                    .spot_market_config_by_index(market_index)
                    .expect("exists");
                (
                    RemainingAccount::Spot {
                        pubkey: *pubkey,
                        writable,
                    },
                    oracle,
                )
            }
            MarketType::Perp => {
                let PerpMarket { pubkey, amm, .. } = program_data
                    .perp_market_config_by_index(market_index)
                    .expect("exists");
                (
                    RemainingAccount::Perp {
                        pubkey: *pubkey,
                        writable,
                    },
                    &amm.oracle,
                )
            }
        };
        if let Err(idx) = accounts.binary_search(&account) {
            accounts.insert(idx, account);
        }
        let oracle = RemainingAccount::Oracle { pubkey: *oracle };
        if let Err(idx) = accounts.binary_search(&oracle) {
            accounts.insert(idx, oracle);
        }
    };

    for MarketId { index, kind } in markets_writable {
        include_market(*index, *kind, true);
    }

    for MarketId { index, kind } in markets_readable {
        include_market(*index, *kind, false);
    }

    for user in users {
        // Drift program performs margin checks which requires reading user positions
        for p in user.spot_positions.iter().filter(|p| !p.is_available()) {
            include_market(p.market_index, MarketType::Spot, false);
        }
        for p in user.perp_positions.iter().filter(|p| !p.is_available()) {
            include_market(p.market_index, MarketType::Perp, false);
        }
    }
    // always manually try to include the quote (USDC) market
    // TODO: this is not exactly the same semantics as the TS sdk
    include_market(MarketId::QUOTE_SPOT.index, MarketType::Spot, false);

    let mut account_metas = base_accounts.to_account_metas(None);
    account_metas.extend(accounts.into_iter().map(Into::into));
    account_metas
}

/// Fetch all market accounts from drift program (does not require `getProgramAccounts` RPC which is often unavailable)
pub async fn get_market_accounts(
    client: &RpcClient,
) -> SdkResult<(Vec<SpotMarket>, Vec<PerpMarket>)> {
    let state_data = client
        .get_account_data(state_account())
        .await
        .expect("state account fetch");
    let state = State::try_deserialize(&mut state_data.as_slice()).expect("state deserializes");
    let spot_market_pdas: Vec<Pubkey> = (0..state.number_of_spot_markets)
        .map(derive_spot_market_account)
        .collect();
    let perp_market_pdas: Vec<Pubkey> = (0..state.number_of_markets)
        .map(derive_perp_market_account)
        .collect();

    let (spot_markets, perp_markets) = tokio::join!(
        client.get_multiple_accounts(spot_market_pdas.as_slice()),
        client.get_multiple_accounts(perp_market_pdas.as_slice())
    );

    let spot_markets = spot_markets?
        .into_iter()
        .map(|x| {
            let account = x.unwrap();
            SpotMarket::try_deserialize(&mut account.data.as_slice()).unwrap()
        })
        .collect();

    let perp_markets = perp_markets?
        .into_iter()
        .map(|x| {
            let account = x.unwrap();
            PerpMarket::try_deserialize(&mut account.data.as_slice()).unwrap()
        })
        .collect();

    Ok((spot_markets, perp_markets))
}

/// Drift wallet
#[derive(Clone, Debug)]
pub struct Wallet {
    /// The signing keypair, it could be authority or delegate
    pub signer: Arc<Keypair>,
    /// The drift 'authority' account
    /// user (sub)accounts are derived from this
    authority: Pubkey,
    /// The drift 'stats' account
    stats: Pubkey,
}

impl Wallet {
    /// Returns true if the wallet is configured for delegated signing
    pub fn is_delegated(&self) -> bool {
        self.authority != self.signer.pubkey() && self.signer.pubkey().is_on_curve()
    }
    /// Init wallet from a string that could be either a file path or the encoded key, uses default sub-account
    pub fn try_from_str(path_or_key: &str) -> SdkResult<Self> {
        let authority = utils::load_keypair_multi_format(path_or_key)?;
        Ok(Self::new(authority))
    }
    /// Construct a read-only wallet
    pub fn read_only(authority: Pubkey) -> Self {
        Self {
            signer: Arc::new(Keypair::from_bytes(&[0_u8; 64]).expect("empty signer")),
            authority,
            stats: Wallet::derive_stats_account(&authority, &constants::PROGRAM_ID),
        }
    }
    /// Init wallet from base58 encoded seed, uses default sub-account
    ///
    /// # panics
    /// if the key is invalid
    pub fn from_seed_bs58(seed: &str) -> Self {
        let authority: Keypair = Keypair::from_base58_string(seed);
        Self::new(authority)
    }
    /// Init wallet from seed bytes, uses default sub-account
    pub fn from_seed(seed: &[u8]) -> SdkResult<Self> {
        let authority: Keypair = keypair_from_seed(seed).map_err(|_| SdkError::InvalidSeed)?;
        Ok(Self::new(authority))
    }
    /// Init wallet with keypair
    ///
    /// `authority` keypair for tx signing
    pub fn new(authority: Keypair) -> Self {
        Self {
            stats: Wallet::derive_stats_account(&authority.pubkey(), &constants::PROGRAM_ID),
            authority: authority.pubkey(),
            signer: Arc::new(authority),
        }
    }
    /// Convert the wallet into a delegated one by providing the `authority` public key
    pub fn to_delegated(&mut self, authority: Pubkey) {
        self.stats = Wallet::derive_stats_account(&authority, &constants::PROGRAM_ID);
        self.authority = authority;
    }
    /// Calculate the address of a drift user account/sub-account
    pub fn derive_user_account(
        authority: &Pubkey,
        sub_account_id: u16,
        program: &Pubkey,
    ) -> Pubkey {
        let (account_drift_pda, _seed) = Pubkey::find_program_address(
            &[
                &b"user"[..],
                authority.as_ref(),
                &sub_account_id.to_le_bytes(),
            ],
            program,
        );
        account_drift_pda
    }

    /// Calculate the address of a drift stats account
    pub fn derive_stats_account(account: &Pubkey, program: &Pubkey) -> Pubkey {
        let (account_drift_pda, _seed) =
            Pubkey::find_program_address(&[&b"user_stats"[..], account.as_ref()], program);
        account_drift_pda
    }

    /// Signs the given tx `message` returning the tx on success
    pub fn sign_tx(
        &self,
        mut message: VersionedMessage,
        recent_block_hash: Hash,
    ) -> SdkResult<VersionedTransaction> {
        message.set_recent_blockhash(recent_block_hash);
        let signer: &dyn Signer = self.signer.as_ref();
        VersionedTransaction::try_new(message, &[signer]).map_err(Into::into)
    }

    /// Return the wallet authority address
    pub fn authority(&self) -> &Pubkey {
        &self.authority
    }
    /// Return the wallet signing address
    pub fn signer(&self) -> Pubkey {
        self.signer.pubkey()
    }
    /// Return the drift user stats address
    pub fn stats(&self) -> &Pubkey {
        &self.stats
    }
    /// Return the address of the default sub-account (0)
    pub fn default_sub_account(&self) -> Pubkey {
        self.sub_account(0)
    }
    /// Calculate the drift user address given a `sub_account_id`
    pub fn sub_account(&self, sub_account_id: u16) -> Pubkey {
        Self::derive_user_account(self.authority(), sub_account_id, &constants::PROGRAM_ID)
    }
}

impl From<Keypair> for Wallet {
    fn from(value: Keypair) -> Self {
        Self::new(value)
    }
}
