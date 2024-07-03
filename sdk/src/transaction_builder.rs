use std::borrow::Cow;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use drift::{
    controller::position::PositionDirection,
    instructions::SpotFulfillmentType,
    math::constants::QUOTE_SPOT_MARKET_INDEX,
    state::{
        order_params::{ModifyOrderParams, OrderParams},
        perp_market::PerpMarket,
        spot_market::SpotMarket,
        state::State,
        user::{MarketType, Order, User},
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    message::{v0, Message, VersionedMessage},
    pubkey::Pubkey,
};

use crate::{
    addresses::pda::get_user_stats_account_pubkey,
    constants::{
        self, derive_perp_market_account, derive_spot_market_account, state_account, ProgramData,
    },
    types::{MakerInfo, MarketId, ReferrerInfo, RemainingAccount, SdkResult, TxParams},
    Wallet,
};

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
    pub(crate) ixs: Vec<Instruction>,
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

    pub fn extend_ix(mut self, ixs: Vec<Instruction>) -> Self {
        self.ixs.extend(ixs);

        self
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

    pub fn force_cancel_orders(
        mut self,
        filler: Pubkey,
        user_account_pubkey: Pubkey,
        user_account: &User,
    ) -> Self {
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::ForceCancelOrder {
                state: *state_account(),
                filler,
                user: user_account_pubkey,
                authority: self.authority,
            },
            &[user_account],
            &[],
            &[MarketId::spot(QUOTE_SPOT_MARKET_INDEX)],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::ForceCancelOrders {}),
        };
        self.ixs.push(ix);

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

    pub fn trigger_order_ix(
        mut self,
        user_account_pubkey: &Pubkey,
        user_account: &User,
        order: &Order,
        filler: Option<&Pubkey>,
        _remaining_accounts: Vec<AccountMeta>,
    ) -> Self {
        let filler = filler.unwrap_or(&self.authority);
        let accounts = build_accounts(
            self.program_data,
            drift::accounts::TriggerOrder {
                state: *state_account(),
                authority: self.authority,
                filler: *filler,
                user: *user_account_pubkey,
            },
            &[user_account],
            &[],
            &[MarketId::perp(order.market_index)],
        );

        // accounts.extend(remaining_accounts);

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::TriggerOrder {
                order_id: order.order_id,
            }),
        };
        self.ixs.push(ix);

        self
    }

    pub fn revert_fill(mut self, filler: Pubkey) -> Self {
        let filler_stats = get_user_stats_account_pubkey(&constants::PROGRAM_ID, filler);

        let accounts = build_accounts(
            self.program_data,
            drift::accounts::RevertFill {
                state: *state_account(),
                authority: self.authority,
                filler,
                filler_stats,
            },
            &[],
            &[],
            &[],
        );

        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::RevertFill {}),
        };
        self.ixs.push(ix);

        self
    }

    // TODO: remaining_accounts
    pub fn fill_perp_order(
        mut self,
        user_account_pubkey: Pubkey,
        user_account: &User,
        order: &Order,
        maker_info: &[MakerInfo],
        _referre_info: &Option<ReferrerInfo>,
    ) -> Self {
        let user_stats_pubkey =
            get_user_stats_account_pubkey(&constants::PROGRAM_ID, user_account.authority);

        let filler = self.account_data.authority;
        let filler_stats_pubkey = get_user_stats_account_pubkey(&constants::PROGRAM_ID, filler);

        let market_index = order.market_index;

        let mut user_accounts = vec![user_account];
        for maker in maker_info {
            user_accounts.push(&maker.maker_user_account);
            user_accounts.push(&maker.maker_user_account);
        }

        let accounts = build_accounts(
            self.program_data,
            drift::accounts::FillOrder {
                state: *state_account(),
                authority: self.authority,
                filler,
                filler_stats: filler_stats_pubkey,
                user: user_account_pubkey,
                user_stats: user_stats_pubkey,
            },
            &user_accounts,
            &[],
            &[MarketId::perp(market_index)],
        );

        let order_id = order.order_id;
        let ix = Instruction {
            program_id: constants::PROGRAM_ID,
            accounts,
            data: InstructionData::data(&drift::instruction::FillPerpOrder {
                order_id: Some(order_id),
                _maker_order_id: None,
            }),
        };
        self.ixs.push(ix);

        self
    }

    pub fn tx_params(self, _tx_params: TxParams) -> Self {
        self
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

    pub fn instructions(&self) -> &[Instruction] {
        &self.ixs
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
