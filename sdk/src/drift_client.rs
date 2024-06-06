use std::{borrow::Cow, collections::HashMap, sync::Arc};

use anchor_lang::{AccountDeserialize, Discriminator};
use drift::{
    math::constants::QUOTE_SPOT_MARKET_INDEX,
    state::{
        oracle::{get_oracle_price, OracleSource},
        perp_market::PerpMarket,
        spot_market::SpotMarket,
        state::State,
        user::{MarketType, Order, OrderStatus, PerpPosition, SpotPosition, User, UserStats},
    },
};
use futures_util::TryFutureExt;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    account_info::IntoAccountInfo,
    address_lookup_table_account::AddressLookupTableAccount,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::VersionedMessage,
    pubkey::Pubkey,
    signature::Signature,
};
use tokio::sync::RwLock;

use crate::{
    blockhash_subscriber::BlockhashSubscriber,
    constants::{
        self, derive_perp_market_account, derive_spot_market_account, market_lookup_table,
        state_account, MarketExt, ProgramData,
    },
    drift_client_config::ClientOpts,
    event_emitter::EventEmitter,
    marketmap::MarketMap,
    oraclemap::{Oracle, OracleMap},
    types::{Context, DataAndSlot, MarketId, SdkError, SdkResult},
    user::DriftUser,
    user_config::UserSubscriptionConfig,
    utils::{self, decode, get_ws_url},
    websocket_account_subscriber::{AccountUpdate, WebsocketAccountSubscriber},
    AccountProvider, TransactionBuilder, Wallet,
};

struct RemainingAccountParams {
    user_accounts: Vec<User>,
    writable_perp_market_indexes: Option<Vec<u16>>,
    writable_spot_market_indexes: Option<Vec<u16>>,
    readable_perp_market_indexes: Option<Vec<u16>>,
    readable_spot_market_indexes: Option<Vec<u16>>,
    use_market_last_slot_cache: Option<bool>,
}

/// Drift Client API
///
/// It is cheaply clone-able and consumers are encouraged to do so
/// It is not recommended to create multiple instances with `::new()` as this will not re-use underlying resources such
/// as network connections or memory allocations
#[derive(Clone)]
#[must_use]
pub struct DriftClient<T, U>
where
    T: AccountProvider,
{
    pub backend: &'static DriftClientBackend<T>,
    pub wallet: Wallet,
    pub active_sub_account_id: u16,
    pub sub_account_ids: Vec<u16>,
    pub users: Vec<DriftUser>,
    pub user_account_subscription_config: Option<UserSubscriptionConfig<U>>,
}

impl<T, U> DriftClient<T, U>
where
    T: AccountProvider,
{
    pub async fn new(context: Context, account_provider: T, wallet: Wallet) -> SdkResult<Self> {
        Self::new_with_opts(context, account_provider, wallet, ClientOpts::default()).await
    }

    pub async fn new_with_opts(
        context: Context,
        account_provider: T,
        wallet: Wallet,
        opts: ClientOpts,
    ) -> SdkResult<Self> {
        Ok(Self {
            backend: Box::leak(Box::new(
                DriftClientBackend::new(context, account_provider).await?,
            )),
            wallet,
            active_sub_account_id: opts.active_sub_account_id(),
            sub_account_ids: opts.sub_account_ids().to_vec(),
            users: vec![],
            user_account_subscription_config: opts.account_subscription(),
        })
    }

    /// Subscribe to the Drift Client Backend
    /// This is a no-op if already subscribed
    pub async fn subscribe(&self) -> SdkResult<()> {
        self.backend.subscribe().await
    }

    /// Unsubscribe from the Drift Client Backend
    /// This is a no-op if not subscribed
    pub async fn unsubscribe(&self) -> SdkResult<()> {
        self.backend.unsubscribe().await
    }

    pub fn fetch_market_lookup_table_account(&self) -> AddressLookupTableAccount {
        self.backend.program_data.lookup_table.clone()
    }

    pub async fn add_user(&mut self, sub_account_id: u16) -> SdkResult<()> {
        let pubkey =
            Wallet::derive_user_account(self.wallet.authority(), sub_account_id, &drift::ID);
        let mut user = DriftUser::new(pubkey, self, Some(sub_account_id)).await?;
        user.subscribe().await?;
        self.users.push(user);
        Ok(())
    }

    pub fn get_user(&self, sub_account_id: Option<u16>) -> Option<&DriftUser> {
        let sub_account_id = sub_account_id.unwrap_or(self.active_sub_account_id);
        self.users
            .iter()
            .find(|u| u.sub_account == Some(sub_account_id))
    }

    /// Get a stats account
    ///
    /// Returns the deserialized account data (`UserStats`)
    pub async fn get_user_stats(&self, authority: &Pubkey) -> SdkResult<UserStats> {
        let user_stats_pubkey = Wallet::derive_stats_account(authority, &constants::PROGRAM_ID);
        self.backend.get_account(&user_stats_pubkey).await
    }

    /// Get the user account data
    ///
    /// `account` the drift user PDA
    ///
    /// Returns the deserialized account data (`User`)
    pub async fn get_user_account(&self, account: &Pubkey) -> SdkResult<User> {
        self.backend.get_account(account).await
    }

    pub fn get_user_account_and_slot(
        &self,
        sub_account_id: Option<u16>,
    ) -> SdkResult<DataAndSlot<User>> {
        let user = self
            .get_user(sub_account_id)
            .ok_or(SdkError::Generic("Not found user".to_string()))?;
        Ok(user.get_user_account_and_slot())
    }

    /// Return a handle to the inner RPC client
    pub fn inner(&self) -> &RpcClient {
        self.backend.client()
    }

    /// Return on-chain program metadata
    pub fn program_data(&self) -> &ProgramData {
        &self.backend.program_data
    }

    /// Get the active sub account id
    pub fn get_sub_account_id_for_ix(&self, sub_account_id: Option<u16>) -> u16 {
        sub_account_id.unwrap_or(self.active_sub_account_id)
    }

    async fn get_remaining_accounts(&self, params: RemainingAccountParams) -> SdkResult<()> {
        let (mut oracle_account_map, mut spot_market_account_map, mut perp_market_account_map) =
            self.get_remaining_account_maps_for_users(&params.user_accounts)
                .await?;

        if let Some(true) = params.use_market_last_slot_cache {
            let last_user_slot = self.get_user_account_and_slot(None)?;
            // for
            for entry in self.backend.perp_market_map.marketmap.iter() {
                let market_index = *entry.key();
                let DataAndSlot { slot, data } = entry.value();
                // if cache has more recent slot than user positions account slot, add market to remaining accounts
                // otherwise remove from slot
                if slot > &last_user_slot.slot {
                    self.add_perp_market_to_remaining_account_maps(
                        data.market_index,
                        false,
                        &mut oracle_account_map,
                        &mut spot_market_account_map,
                        &mut perp_market_account_map,
                    )
                    .await?;
                } else {
                    self.backend.perp_market_map.marketmap.remove(&market_index);
                }
            }

            for entry in self.backend.spot_market_map.marketmap.iter() {
                let market_index = *entry.key();
                let DataAndSlot { slot, data } = entry.value();
                // if cache has more recent slot than user positions account slot, add market to remaining accounts
                // otherwise remove from slot
                if slot > &last_user_slot.slot {
                    self.add_spot_market_to_remaining_account_maps(
                        data.market_index,
                        false,
                        &mut oracle_account_map,
                        &mut spot_market_account_map,
                    )
                    .await?;
                } else {
                    self.backend.perp_market_map.marketmap.remove(&market_index);
                }
            }
        }

        if let Some(perp_indexes) = params.readable_perp_market_indexes {
            for index in perp_indexes {
                self.add_perp_market_to_remaining_account_maps(
                    index,
                    false,
                    &mut oracle_account_map,
                    &mut spot_market_account_map,
                    &mut perp_market_account_map,
                )
                .await?;
            }
        }

        // TODO
        // https://github.com/drift-labs/protocol-v2/blob/507e79afb919662f9872405246599977ab6d93dd/sdk/src/driftClient.ts#L1565

        Ok(())
    }

    async fn add_perp_market_to_remaining_account_maps(
        &self,
        market_index: u16,
        writable: bool,
        oracle_account_map: &mut HashMap<Pubkey, AccountMeta>,
        spot_market_account_map: &mut HashMap<u16, AccountMeta>,
        perp_market_account_map: &mut HashMap<u16, AccountMeta>,
    ) -> SdkResult<()> {
        let perp_market_account = self.get_perp_market_info(market_index).await?;
        perp_market_account_map.insert(
            market_index,
            AccountMeta {
                pubkey: perp_market_account.pubkey,
                is_signer: false,
                is_writable: writable,
            },
        );
        let oracle_writable =
            writable && perp_market_account.amm.oracle_source == OracleSource::Prelaunch;
        oracle_account_map.insert(
            perp_market_account.amm.oracle,
            AccountMeta {
                pubkey: perp_market_account.amm.oracle,
                is_signer: false,
                is_writable: oracle_writable,
            },
        );
        self.add_spot_market_to_remaining_account_maps(
            perp_market_account.quote_spot_market_index,
            false,
            oracle_account_map,
            spot_market_account_map,
        )
        .await?;

        Ok(())
    }

    async fn add_spot_market_to_remaining_account_maps(
        &self,
        market_index: u16,
        writable: bool,
        oracle_account_map: &mut HashMap<Pubkey, AccountMeta>,
        spot_market_account_map: &mut HashMap<u16, AccountMeta>,
    ) -> SdkResult<()> {
        let spot_market_account = self.get_spot_market_info(market_index).await?;
        spot_market_account_map.insert(
            spot_market_account.market_index,
            AccountMeta {
                pubkey: spot_market_account.pubkey,
                is_signer: false,
                is_writable: writable,
            },
        );

        if spot_market_account.oracle == Pubkey::default() {
            oracle_account_map.insert(
                spot_market_account.oracle,
                AccountMeta {
                    pubkey: spot_market_account.oracle,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }

        Ok(())
    }

    /// Get remaining account maps for users
    async fn get_remaining_account_maps_for_users(
        &self,
        user_accounts: &[User],
    ) -> SdkResult<(
        HashMap<Pubkey, AccountMeta>,
        HashMap<u16, AccountMeta>,
        HashMap<u16, AccountMeta>,
    )> {
        let mut oracle_account_map = HashMap::new();
        let mut spot_market_account_map = HashMap::new();
        let mut perp_market_account_map = HashMap::new();

        for user in user_accounts {
            for spot_position in user.spot_positions {
                if !spot_position.is_available() {
                    self.add_spot_market_to_remaining_account_maps(
                        spot_position.market_index,
                        false,
                        &mut oracle_account_map,
                        &mut spot_market_account_map,
                    )
                    .await?;

                    if !spot_position.open_asks == 0 || !spot_position.open_bids == 0 {
                        self.add_spot_market_to_remaining_account_maps(
                            QUOTE_SPOT_MARKET_INDEX,
                            false,
                            &mut oracle_account_map,
                            &mut spot_market_account_map,
                        )
                        .await?;
                    }
                }
            }

            for perp_position in user.perp_positions {
                if !perp_position.is_available() {
                    self.add_perp_market_to_remaining_account_maps(
                        perp_position.market_index,
                        false,
                        &mut oracle_account_map,
                        &mut spot_market_account_map,
                        &mut perp_market_account_map,
                    )
                    .await?;
                }
            }
        }

        Ok((
            oracle_account_map,
            spot_market_account_map,
            perp_market_account_map,
        ))
    }

    /// Get an account's open order by id
    ///
    /// `account` the drift user PDA
    pub async fn get_order_by_id(
        &self,
        account: &Pubkey,
        order_id: u32,
    ) -> SdkResult<Option<Order>> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok(user.orders.iter().find(|o| o.order_id == order_id).copied())
    }

    /// Get an account's open order by user assigned id
    ///
    /// `account` the drift user PDA
    pub async fn get_order_by_user_id(
        &self,
        account: &Pubkey,
        user_order_id: u8,
    ) -> SdkResult<Option<Order>> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok(user
            .orders
            .iter()
            .find(|o| o.user_order_id == user_order_id)
            .copied())
    }

    /// Get all the account's open orders
    ///
    /// `account` the drift user PDA
    pub async fn all_orders(&self, account: &Pubkey) -> SdkResult<Vec<Order>> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok(user
            .orders
            .iter()
            .filter(|o| o.status == OrderStatus::Open)
            .copied()
            .collect())
    }

    /// Get all the account's active positions
    ///
    /// `account` the drift user PDA
    pub async fn all_positions(
        &self,
        account: &Pubkey,
    ) -> SdkResult<(Vec<SpotPosition>, Vec<PerpPosition>)> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok((
            user.spot_positions
                .iter()
                .filter(|s| !s.is_available())
                .copied()
                .collect(),
            user.perp_positions
                .iter()
                .filter(|p| p.is_open_position())
                .copied()
                .collect(),
        ))
    }

    /// Get a perp position by market
    ///
    /// `account` the drift user PDA
    ///
    /// Returns the position if it exists
    pub async fn perp_position(
        &self,
        account: &Pubkey,
        market_index: u16,
    ) -> SdkResult<Option<PerpPosition>> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok(user
            .perp_positions
            .iter()
            .find(|p| p.market_index == market_index && !p.is_available())
            .copied())
    }

    /// Get a spot position by market
    ///
    /// `account` the drift user PDA
    ///
    /// Returns the position if it exists
    pub async fn spot_position(
        &self,
        account: &Pubkey,
        market_index: u16,
    ) -> SdkResult<Option<SpotPosition>> {
        let user = self.backend.get_account::<User>(account).await?;

        Ok(user
            .spot_positions
            .iter()
            .find(|p| p.market_index == market_index && !p.is_available())
            .copied())
    }

    /// Return the DriftClient's wallet
    pub fn wallet(&self) -> &Wallet {
        &self.wallet
    }

    /// Get the latest recent_block_hash
    pub async fn get_latest_blockhash(&self) -> SdkResult<Hash> {
        self.backend
            .client()
            .get_latest_blockhash()
            .await
            .map_err(SdkError::Rpc)
    }

    /// Sign and send a tx to the network
    ///
    /// Returns the signature on success
    pub async fn sign_and_send(&self, tx: VersionedMessage) -> SdkResult<Signature> {
        self.backend
            .sign_and_send(self.wallet(), tx)
            .await
            .map_err(|err| err.to_out_of_sol_error().unwrap_or(err))
    }

    /// Sign and send a tx to the network
    ///
    /// Returns the signature on success
    pub async fn sign_and_send_with_config(
        &self,
        tx: VersionedMessage,
        config: RpcSendTransactionConfig,
    ) -> SdkResult<Signature> {
        self.backend
            .sign_and_send_with_config(self.wallet(), tx, config)
            .await
            .map_err(|err| err.to_out_of_sol_error().unwrap_or(err))
    }

    /// Get live info of a spot market
    pub async fn get_spot_market_info(&self, market_index: u16) -> SdkResult<SpotMarket> {
        let market = derive_spot_market_account(market_index);
        self.backend.get_account(&market).await
    }

    /// Get live info of a perp market
    pub async fn get_perp_market_info(&self, market_index: u16) -> SdkResult<PerpMarket> {
        let market = derive_perp_market_account(market_index);
        self.backend.get_account(&market).await
    }

    /// Lookup a market by symbol
    ///
    /// This operation is not free so lookups should be reused/cached by the caller
    ///
    /// Returns None if symbol does not map to any known market
    pub fn market_lookup(&self, symbol: &str) -> Option<MarketId> {
        if symbol.contains('-') {
            let markets = self.program_data().perp_market_configs();
            if let Some(market) = markets
                .iter()
                .find(|m| m.symbol().eq_ignore_ascii_case(symbol))
            {
                return Some(MarketId::perp(market.market_index));
            }
        } else {
            let markets = self.program_data().spot_market_configs();
            if let Some(market) = markets
                .iter()
                .find(|m| m.symbol().eq_ignore_ascii_case(symbol))
            {
                return Some(MarketId::spot(market.market_index));
            }
        }

        None
    }

    /// Get live oracle price for `market`
    pub async fn oracle_price(&self, market: MarketId) -> SdkResult<i64> {
        self.backend.oracle_price(market).await
    }

    /// Initialize a transaction given a (sub)account address
    ///
    /// ```ignore
    /// let tx = client
    ///     .init_tx(&wallet.sub_account(3), false)
    ///     .cancel_all_orders()
    ///     .place_orders(...)
    ///     .build();
    /// ```
    /// Returns a `TransactionBuilder` for composing the tx
    pub fn init_tx(&self, account: &Pubkey, delegated: bool) -> SdkResult<TransactionBuilder> {
        let user = self.get_user(Some(self.active_sub_account_id));

        match user {
            Some(user) => {
                let account_data = user.get_user_account();
                Ok(TransactionBuilder::new(
                    self.program_data(),
                    *account,
                    Cow::Owned(account_data),
                    delegated,
                ))
            }
            None => Err(SdkError::Generic("user".to_string())),
        }
    }

    pub async fn get_recent_priority_fees(
        &self,
        writable_markets: &[MarketId],
        window: Option<usize>,
    ) -> SdkResult<Vec<u64>> {
        self.backend
            .get_recent_priority_fees(writable_markets, window)
            .await
    }

    pub fn get_state_account(&self) -> Arc<std::sync::RwLock<State>> {
        self.backend.state_account.clone()
    }

    pub fn get_perp_market_account_and_slot(
        &self,
        market_index: u16,
    ) -> Option<DataAndSlot<PerpMarket>> {
        self.backend.get_perp_market_account_and_slot(market_index)
    }

    pub fn get_spot_market_account_and_slot(
        &self,
        market_index: u16,
    ) -> Option<DataAndSlot<SpotMarket>> {
        self.backend.get_spot_market_account_and_slot(market_index)
    }

    pub fn get_perp_market_account(&self, market_index: u16) -> Option<PerpMarket> {
        self.backend
            .get_perp_market_account_and_slot(market_index)
            .map(|x| x.data)
    }

    pub fn get_perp_market_accounts(&self) -> Vec<PerpMarket> {
        self.backend.get_perp_market_accounts()
    }

    pub fn get_spot_market_account(&self, market_index: u16) -> Option<SpotMarket> {
        self.backend
            .get_spot_market_account_and_slot(market_index)
            .map(|x| x.data)
    }

    pub fn num_perp_markets(&self) -> usize {
        self.backend.num_perp_markets()
    }

    pub fn num_spot_markets(&self) -> usize {
        self.backend.num_spot_markets()
    }

    pub fn get_oracle_price_data_and_slot(&self, oracle_pubkey: &Pubkey) -> Option<Oracle> {
        self.backend.get_oracle_price_data_and_slot(oracle_pubkey)
    }

    pub fn get_oracle_price_data_and_slot_for_perp_market(
        &self,
        market_index: u16,
    ) -> Option<Oracle> {
        self.backend
            .get_oracle_price_data_and_slot_for_perp_market(market_index)
    }

    pub fn get_oracle_price_data_and_slot_for_spot_market(
        &self,
        market_index: u16,
    ) -> Option<Oracle> {
        self.backend
            .get_oracle_price_data_and_slot_for_spot_market(market_index)
    }

    pub async fn get_update_funding_rate_ix(
        &self,
        perp_market_index: u16,
        oracle_pubkey: &Pubkey,
    ) -> SdkResult<Instruction> {
        let perp_market = self.get_perp_market_info(perp_market_index).await?;
        let account_data: User = self
            .backend
            .get_account(&self.wallet.default_sub_account())
            .await?;

        let ix = &TransactionBuilder::new(
            self.program_data(),
            self.wallet.default_sub_account(),
            Cow::Owned(account_data),
            false,
        )
        .update_funding_rate(perp_market_index, &perp_market.pubkey, oracle_pubkey)
        .ixs[0];

        Ok(ix.clone())
    }
}

/// Provides the heavy-lifting and network facing features of the SDK
/// It is intended to be a singleton
pub struct DriftClientBackend<T: AccountProvider> {
    pub rpc_client: Arc<RpcClient>,
    pub account_provider: T,
    pub program_data: ProgramData,
    pub perp_market_map: MarketMap<PerpMarket>,
    pub spot_market_map: MarketMap<SpotMarket>,
    pub oracle_map: Arc<OracleMap>,
    pub state_account: Arc<std::sync::RwLock<State>>,
    pub blockhash_subscriber: Arc<RwLock<BlockhashSubscriber>>,
}

impl<T: AccountProvider> DriftClientBackend<T> {
    /// Initialize a new `DriftClientBackend`
    async fn new(context: Context, account_provider: T) -> SdkResult<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            account_provider.endpoint(),
            account_provider.commitment_config(),
        );

        let perp_market_map = MarketMap::<PerpMarket>::new(
            account_provider.commitment_config(),
            account_provider.endpoint(),
            true,
        );
        let spot_market_map = MarketMap::<SpotMarket>::new(
            account_provider.commitment_config(),
            account_provider.endpoint(),
            true,
        );

        let lookup_table_address = market_lookup_table(context);

        let (_, _, lut, state) = tokio::try_join!(
            perp_market_map.sync(),
            spot_market_map.sync(),
            rpc_client
                .get_account(&lookup_table_address)
                .map_err(Into::into),
            rpc_client.get_account(state_account()).map_err(Into::into),
        )?;
        let lookup_table = utils::deserialize_alt(lookup_table_address, &lut)?;

        let perp_oracles = perp_market_map.oracles();
        let spot_oracles = spot_market_map.oracles();

        let oracle_map = OracleMap::new(
            account_provider.commitment_config(),
            account_provider.endpoint(),
            true,
            perp_oracles,
            spot_oracles,
        );

        let blockhash_subscriber = Arc::new(RwLock::new(BlockhashSubscriber::new(
            2,
            account_provider.endpoint(),
        )));

        Ok(Self {
            rpc_client: Arc::new(rpc_client),
            account_provider,
            program_data: ProgramData::new(
                spot_market_map.values(),
                perp_market_map.values(),
                lookup_table,
            ),
            perp_market_map,
            spot_market_map,
            oracle_map: Arc::new(oracle_map),
            state_account: Arc::new(std::sync::RwLock::new(
                State::try_deserialize(&mut state.data.as_ref()).expect("valid state"),
            )),
            blockhash_subscriber,
        })
    }

    async fn subscribe(&self) -> SdkResult<()> {
        tokio::try_join!(
            self.perp_market_map.subscribe(),
            self.spot_market_map.subscribe(),
            self.oracle_map.subscribe(),
            self.state_subscribe(),
            BlockhashSubscriber::subscribe(self.blockhash_subscriber.clone()),
        )?;
        Ok(())
    }

    async fn unsubscribe(&self) -> SdkResult<()> {
        tokio::try_join!(
            self.perp_market_map.unsubscribe(),
            self.spot_market_map.unsubscribe(),
            self.oracle_map.unsubscribe(),
        )?;
        Ok(())
    }

    async fn state_subscribe(&self) -> SdkResult<()> {
        let pubkey = *state_account();

        let mut subscription: WebsocketAccountSubscriber<State> = WebsocketAccountSubscriber::new(
            "state",
            get_ws_url(&self.rpc_client.url()).expect("valid url"),
            pubkey,
            self.rpc_client.commitment(),
            EventEmitter::new(),
        );

        let state = self.state_account.clone();

        subscription.event_emitter.subscribe("state", move |event| {
            if let Some(update) = event.as_any().downcast_ref::<AccountUpdate>() {
                let new_data = decode::<State>(update.data.data.clone()).expect("valid state data");
                let mut state_writer = state.write().unwrap();
                *state_writer = new_data;
            }
        });

        subscription.subscribe().await?;

        Ok(())
    }

    fn get_perp_market_account_and_slot(
        &self,
        market_index: u16,
    ) -> Option<DataAndSlot<PerpMarket>> {
        self.perp_market_map.get(&market_index)
    }

    fn get_spot_market_account_and_slot(
        &self,
        market_index: u16,
    ) -> Option<DataAndSlot<SpotMarket>> {
        self.spot_market_map.get(&market_index)
    }

    fn get_perp_market_accounts(&self) -> Vec<PerpMarket> {
        self.perp_market_map.values()
    }

    fn num_perp_markets(&self) -> usize {
        self.perp_market_map.size()
    }

    fn num_spot_markets(&self) -> usize {
        self.spot_market_map.size()
    }

    fn get_oracle_price_data_and_slot(&self, oracle_pubkey: &Pubkey) -> Option<Oracle> {
        self.oracle_map.get(oracle_pubkey)
    }

    fn get_oracle_price_data_and_slot_for_perp_market(&self, market_index: u16) -> Option<Oracle> {
        let market = self.get_perp_market_account_and_slot(market_index)?;

        let oracle = market.data.amm.oracle;
        let current_oracle = self
            .oracle_map
            .current_perp_oracle(market_index)
            .expect("oracle");

        if oracle != current_oracle {
            let source = market.data.amm.oracle_source;
            let clone = self.oracle_map.clone();
            tokio::task::spawn_local(async move {
                let _ = clone.add_oracle(oracle, source).await;
                clone.update_perp_oracle(market_index, oracle)
            });
        }

        self.get_oracle_price_data_and_slot(&current_oracle)
    }

    fn get_oracle_price_data_and_slot_for_spot_market(&self, market_index: u16) -> Option<Oracle> {
        let market = self.get_spot_market_account_and_slot(market_index)?;

        let oracle = market.data.oracle;
        let current_oracle = self
            .oracle_map
            .current_spot_oracle(market_index)
            .expect("oracle");

        if oracle != current_oracle {
            let source = market.data.oracle_source;
            let clone = self.oracle_map.clone();
            tokio::task::spawn_local(async move {
                let _ = clone.add_oracle(oracle, source).await;
                clone.update_spot_oracle(market_index, oracle);
            });
        }

        self.get_oracle_price_data_and_slot(&market.data.oracle)
    }

    /// Return a handle to the inner RPC client
    fn client(&self) -> &RpcClient {
        &self.rpc_client
    }

    /// Get recent tx priority fees
    ///
    /// - `window` # of slots to include in the fee calculation
    async fn get_recent_priority_fees(
        &self,
        writable_markets: &[MarketId],
        window: Option<usize>,
    ) -> SdkResult<Vec<u64>> {
        let addresses: Vec<Pubkey> = writable_markets
            .iter()
            .filter_map(|x| match x.kind {
                MarketType::Spot => self
                    .program_data
                    .spot_market_config_by_index(x.index)
                    .map(|x| x.pubkey),
                MarketType::Perp => self
                    .program_data
                    .perp_market_config_by_index(x.index)
                    .map(|x| x.pubkey),
            })
            .collect();

        let response = self
            .rpc_client
            .get_recent_prioritization_fees(addresses.as_slice())
            .await?;
        let window = window.unwrap_or(5).max(1);
        let fees = response
            .iter()
            .take(window)
            .map(|x| x.prioritization_fee)
            .collect();

        Ok(fees)
    }

    /// Get all drift program accounts by Anchor type
    #[allow(dead_code)]
    async fn get_program_accounts<U: AccountDeserialize + Discriminator>(
        &self,
    ) -> SdkResult<Vec<U>> {
        let accounts = self
            .rpc_client
            .get_program_accounts_with_config(
                &constants::PROGRAM_ID,
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        U::DISCRIMINATOR.to_vec(),
                    ))]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64Zstd),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await?;

        accounts
            .iter()
            .map(|(_, account_data)| {
                U::try_deserialize(&mut account_data.data.as_ref())
                    .map_err(|err| SdkError::Anchor(Box::new(err)))
            })
            .collect()
    }

    /// Fetch an `account` as an Anchor account type
    async fn get_account<U: AccountDeserialize>(&self, account: &Pubkey) -> SdkResult<U> {
        let account_data = self.account_provider.get_account(*account).await?;
        U::try_deserialize(&mut account_data.data.as_ref()).map_err(|_err| SdkError::InvalidAccount)
    }

    /// Sign and send a tx to the network
    ///
    /// Returns the signature on success
    pub async fn sign_and_send(
        &self,
        wallet: &Wallet,
        tx: VersionedMessage,
    ) -> SdkResult<Signature> {
        let blockhash_reader = self.blockhash_subscriber.read().await;
        drop(blockhash_reader);
        let recent_block_hash = self
            .rpc_client
            .get_latest_blockhash()
            .await
            .expect("get recent blockhash");
        let tx = wallet.sign_tx(tx, recent_block_hash)?;
        self.rpc_client
            .send_transaction(&tx)
            .await
            .map_err(|err| err.into())
    }

    /// Sign and send a tx to the network with custom send config
    /// allows setting commitment level, retries, etc.
    ///
    /// Returns the signature on success
    pub async fn sign_and_send_with_config(
        &self,
        wallet: &Wallet,
        tx: VersionedMessage,
        config: RpcSendTransactionConfig,
    ) -> SdkResult<Signature> {
        let blockhash_reader = self.blockhash_subscriber.read().await;
        let recent_block_hash = blockhash_reader.get_valid_blockhash();
        drop(blockhash_reader);
        let tx = wallet.sign_tx(tx, recent_block_hash)?;
        self.rpc_client
            .send_transaction_with_config(&tx, config)
            .await
            .map_err(|err| err.into())
    }

    /// Fetch the live oracle price for `market`
    pub async fn oracle_price(&self, market: MarketId) -> SdkResult<i64> {
        let (oracle, oracle_source) = match market.kind {
            MarketType::Perp => {
                let market = self
                    .program_data
                    .perp_market_config_by_index(market.index)
                    .ok_or(SdkError::InvalidOracle)?;
                (market.amm.oracle, market.amm.oracle_source)
            }
            MarketType::Spot => {
                let market = self
                    .program_data
                    .spot_market_config_by_index(market.index)
                    .ok_or(SdkError::InvalidOracle)?;
                (market.oracle, market.oracle_source)
            }
        };

        let (current_slot, oracle_account) = tokio::join!(
            self.rpc_client.get_slot(),
            self.account_provider.get_account(oracle)
        );
        let price_data = get_oracle_price(
            &oracle_source,
            &(oracle, oracle_account?).into_account_info(),
            current_slot?,
        )
        .unwrap();
        Ok(price_data.price)
    }
}
