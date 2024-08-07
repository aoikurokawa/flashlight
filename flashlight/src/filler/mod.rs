use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use drift::state::{
    oracle::OracleSource,
    perp_market::PerpMarket,
    user::{MarketType, OrderType, User},
};
use log::info;
use lru::LruCache;
use rand::{seq::SliceRandom, thread_rng};
use sdk::{
    accounts::BulkAccountLoader,
    blockhash_subscriber::BlockhashSubscriber,
    clock::clock_subscriber::ClockSubscriber,
    constants,
    dlob::{
        dlob::{MarketAccount, NodeToFill, DLOB},
        dlob_node::{DLOBNode, Node, NodeType},
        dlob_subscriber::DLOBSubscriber,
        types::{DLOBSubscriptionConfig, DlobSource, SlotSource},
    },
    drift_client::DriftClient,
    jupiter::JupiterClient,
    math::{
        market::{calculate_ask_price, calculate_bid_price},
        oracle::is_oracle_valid,
        order::{is_fillable_by_vamm, is_order_expired},
    },
    priority_fee::priority_fee_subscriber::PriorityFeeSubscriber,
    slot_subscriber::SlotSubscriber,
    types::{MakerInfo, ReferrerInfo},
    usermap::{user_stats_map::UserStatsMap, UserMap},
    AccountProvider,
};
use solana_client::nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{option_serializer::OptionSerializer, UiTransactionEncoding};

use crate::{
    bundle_sender::BundleSender,
    common::tx_log_parse::{
        is_end_ix_log, is_err_filling_log, is_err_stale_oracle, is_fill_ix_log, is_ix_log,
        is_maker_breached_maintainance_margin_log, is_order_does_not_exist_log,
        is_taker_breached_maintainance_margin_log,
    },
    config::{FillerConfig, GlobalConfig},
    maker_selection::select_makers,
    metrics::RuntimeSpec,
    types::JitoStrategy,
    util::{
        get_fill_signature_from_user_account_and_orader_id, get_node_to_fill_signature,
        get_node_to_trigger_signature, get_transaction_account_metas, simulate_and_get_tx_with_cus,
        valid_minimum_gas_amount, valid_rebalance_settled_pnl_threshold,
        SimulateAndGetTxWithCUsParams, SimulateAndGetTxWithCUsResponse,
    },
};

use self::pending_tx_sigs_to_confirm::{PendingTxSigsToconfirm, TxType};

mod pending_tx_sigs_to_confirm;

const MAX_TX_PACK_SIZE: usize = 1230; //1232;
const CU_PER_FILL: usize = 260_000; // CU cost for a successful fill
const BURST_CU_PER_FILL: usize = 350_000; // CU cost for a successful fill
const MAX_CU_PER_TX: usize = 1_400_000; // seems like this is all budget program gives us...on devnet
const TX_COUNT_COOLDOWN_ON_BURST: u16 = 10; // send this many tx before resetting burst mode
const DEFAULT_INTERVAL_MS: u16 = 6000;
const FILL_ORDER_THROTTLE_BACKOFF: u64 = 1000; // the time to wait before trying to fill a throttled (error filling) node again
const THROTTLED_NODE_SIZE_TO_PRUNE: usize = 10; // Size of throttled nodes to get to before pruning the map
const TRIGGER_ORDER_COOLDOWN_MS: u64 = 1000; // the time to wait before trying to a node in the triggering map again
pub(crate) const MAX_MAKERS_PER_FILL: usize = 6; // max number of unique makers to include per fill
const MAX_ACCOUNTS_PER_TX: usize = 64; // solana limit, track https://github.com/solana-labs/solana/issues/27241

const SIM_CU_ESTIMATE_MULTIPLIER: f64 = 1.15;
const SLOTS_UNTIL_JITO_LEADER_TO_SEND: u64 = 4;
const TX_CONFIRMATION_BATCH_SIZE: usize = 100;
const TX_TIMEOUT_THRESHOLD_MS: u128 = 60_000; // tx considered stale after this time and give up confirming
const CONFIRM_TX_RATE_LIMIT_BACKOFF_MS: u64 = 5_000; // wait this long until trying to confirm tx again if rate limited

const EXPIRE_ORDER_BUFFER_SEC: i64 = 60; // add extra time before trying to expire orders (want to avoid 6252 error due to clock drift)

#[allow(dead_code)]
pub struct FillerBot<'a, T>
where
    T: AccountProvider,
{
    name: String,
    dry_run: bool,
    // default_interval_ms: u16,
    slot_subscriber: SlotSubscriber,
    clock_subscriber: ClockSubscriber,
    bulk_account_loader: Option<BulkAccountLoader>,
    // user_stats_map_subscription_config: &'a UserSubscriptionConfig<U>,
    drift_client: Arc<DriftClient<T>>,
    /// Connection to use specifically for confirming transactions
    tx_confirmation_connection: Arc<RpcClient>,
    polling_interval_ms: u16,
    revert_on_failure: Option<bool>,
    simulate_tx_for_cu_estimate: Option<bool>,
    lookup_table_account: Option<AddressLookupTableAccount>,
    bundle_sender: Option<BundleSender>,

    filler_config: FillerConfig,
    global_config: GlobalConfig,
    dlob_subscriber: Option<DLOBSubscriber<T>>,

    user_map: Option<UserMap>,
    user_stats_map: Option<UserStatsMap<T>>,

    // periodic_task_mutex = new Mutex();

    // watchdogTimerMutex = new Mutex();
    watchdog_timer_last_pat_time: Instant,

    interval_ids: Vec<Instant>,
    throttled_nodes: HashMap<String, Instant>,
    filling_nodes: HashMap<String, Instant>,
    triggering_nodes: HashMap<String, Instant>,

    use_burst_cu_limit: bool,
    fill_tx_since_burst_cu: u16,
    fill_tx_id: u16,
    last_settle_pnl: Instant,

    priority_fee_subscriber: PriorityFeeSubscriber<T>,
    blockhash_subscriber: BlockhashSubscriber,
    /// stores txSigs that need to been confirmed in a slower loop, and the time they were confirmed
    pending_tx_sigs_toconfirm: LruCache<Signature, PendingTxSigsToconfirm>,
    expired_nodes_set: LruCache<String, bool>,
    confirm_loop_running: bool,
    confirm_loop_rate_limit_ts: Instant,

    jupiter_client: Option<JupiterClient<'a>>,

    // metrics
    // metrics_initialized: bool,
    // metrics_port: Option<u16>,
    // metrics: Option<Metrics>,
    // boot_time_ms: Option<u16>,
    runtime_spec: RuntimeSpec,
    // runtime_specs_gauge: Option<GaugeValue>,
    // try_fill_duration_histogram: Option<HistogramValue>,
    // est_tx_cu_histogram: Option<HistogramValue>,
    // simulate_tx_histogram: Option<HistogramValue>,
    // last_try_fill_time_gauge: Option<GaugeValue>,
    // mutex_busy_counter: Option<CounterValue>,
    // sent_txs_counter: Option<CounterValue>,
    // attempted_triggers_counter: Option<CounterValue>,
    // landed_txs_counter: Option<CounterValue>,
    // tx_sim_error_counter: Option<CounterValue>,
    // pending_tx_sigs_to_confirm_gauge: Option<GaugeValue>,
    // pending_tx_sigs_loop_rate_limited_counter: Option<CounterValue>,
    // evicted_pending_tx_sigs_to_confirm_counter: Option<CounterValue>,
    // expired_nodes_set_size: Option<GaugeValue>,
    // jito_bundles_accepted_gauge: Option<GaugeValue>,
    // jito_bundles_simulation_failure_gauge: Option<GaugeValue>,
    // jito_dropped_bundle_gauge: Option<GaugeValue>,
    // jito_landed_tips_gauge: Option<GaugeValue>,
    // jito_bundle_count: Option<GaugeValue>,
    has_enough_sol_to_fill: bool,
    rebalance_filler: bool,
    min_gas_balance_to_fill: f64,
    rebalance_settled_pnl_threshold: f64,
}

#[allow(dead_code)]
impl<'a, T> FillerBot<'a, T>
where
    T: AccountProvider + Clone,
{
    pub async fn new(
        websocket_url: &str,
        slot_subscriber: SlotSubscriber,
        bulk_account_loader: Option<BulkAccountLoader>,
        drift_client: Arc<DriftClient<T>>,
        user_map: UserMap,
        runtime_spec: RuntimeSpec,
        global_config: GlobalConfig,
        filler_config: FillerConfig,
        mut priority_fee_subscriber: PriorityFeeSubscriber<T>,
        blockhash_subscriber: BlockhashSubscriber,
        bundle_sender: Option<BundleSender>,
    ) -> Self {
        let drift_client = drift_client.clone();
        // let tx_confirmation_connection = match global_config.tx_confirmation_endpoint {
        //     Some(ref endpoint) => RpcClient::new(endpoint.to_string()),
        //     None => drift_client.backend.rpc_client,
        // };

        // let user_stats_map_subscription_config = match bulk_account_loader {
        //     Some(ref account_loader) => {
        //         // let loader = BulkAccountLoader::new(account_leader.rpc_client, account_leader.commitment, polling_frequency);
        //         UserSubscriptionConfig::Polling {
        //             account_loader: account_loader.clone(),
        //         }
        //     }
        //     None => drift_client
        //         .user_account_subscription_config
        //         .unwrap(),
        // };
        let tx_confirmation_connection =
            if let Some(ref endpoint) = global_config.tx_confirmation_endpoint {
                Arc::new(RpcClient::new(endpoint.to_string()))
            } else {
                drift_client.backend.rpc_client.clone()
            };

        info!(
            "{}: revert_on_failure: {}, simulate_tx_for_cu_estimate: {}",
            filler_config.base_config.bot_id,
            filler_config.revert_on_failure.unwrap_or(true),
            filler_config.simulate_tx_for_cu_estimate.unwrap_or(true),
        );

        info!(
            "{}: jito enabled: {}",
            filler_config.base_config.bot_id,
            bundle_sender.is_some()
        );

        let jupiter_client = if filler_config.rebalance_filler.is_some()
            && runtime_spec.drift_env == "mainnet-beta"
        {
            let client = JupiterClient::new(&drift_client.backend.rpc_client, None);
            Some(client)
        } else {
            None
        };

        info!(
            "{}: rebalancing enabled: {}",
            filler_config.base_config.bot_id,
            jupiter_client.is_some()
        );

        let min_gas_balance_to_fill =
            if !valid_minimum_gas_amount(filler_config.min_gas_balance_to_fill) {
                0.2 * LAMPORTS_PER_SOL as f64
            } else {
                filler_config.min_gas_balance_to_fill.unwrap() * LAMPORTS_PER_SOL as f64
            };

        let rebalance_settled_pnl_threshold = if !valid_rebalance_settled_pnl_threshold(
            filler_config.rebalance_settled_pnl_threshold,
        ) {
            20_f64
        } else {
            filler_config.rebalance_settled_pnl_threshold.unwrap()
        };

        info!(
            "{}: minimum_amount_to_fill: {}",
            filler_config.base_config.bot_id, min_gas_balance_to_fill
        );

        info!(
            "{}: minimum_amount_to_settle: {}",
            filler_config.base_config.bot_id, rebalance_settled_pnl_threshold
        );

        // Openbook SOL/USDC
        // sol-perp
        priority_fee_subscriber.update_addresses(&[
            Pubkey::from_str("8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6").unwrap(),
            Pubkey::from_str("8UJgxaiQx5nTrdDgph5FiahMmzduuLTLf5WmsPegYA6W").unwrap(),
        ]);

        let pubsub_client = PubsubClient::new(websocket_url)
            .await
            .expect("init pubsub client");

        Self {
            global_config,
            filler_config: filler_config.clone(),
            name: filler_config.base_config.bot_id,
            dry_run: filler_config.base_config.dry_run,
            slot_subscriber,
            drift_client,
            tx_confirmation_connection,
            clock_subscriber: ClockSubscriber::new(Arc::new(pubsub_client), None),
            // tx_confirmation_connection,
            bulk_account_loader,
            // user_stats_map_subscription_config: &user_stats_map_subscription_config,
            runtime_spec,
            polling_interval_ms: filler_config
                .filler_polling_interval
                .unwrap_or(DEFAULT_INTERVAL_MS),
            user_map: Some(user_map),
            revert_on_failure: Some(filler_config.revert_on_failure.unwrap_or(true)),
            simulate_tx_for_cu_estimate: Some(
                filler_config.simulate_tx_for_cu_estimate.unwrap_or(true),
            ),
            bundle_sender,
            jupiter_client,
            rebalance_filler: filler_config.rebalance_filler.unwrap_or(false),
            min_gas_balance_to_fill,
            rebalance_settled_pnl_threshold,
            priority_fee_subscriber,
            blockhash_subscriber,
            pending_tx_sigs_toconfirm: LruCache::new(NonZeroUsize::new(10_000).unwrap()),
            expired_nodes_set: LruCache::new(NonZeroUsize::new(100).unwrap()),
            confirm_loop_running: false,
            confirm_loop_rate_limit_ts: Instant::now() - Duration::from_secs(5_000),
            dlob_subscriber: None,
            fill_tx_id: 0,
            fill_tx_since_burst_cu: 0,
            filling_nodes: HashMap::new(),
            has_enough_sol_to_fill: false,
            interval_ids: vec![],
            last_settle_pnl: Instant::now() - Duration::from_secs(60_000),
            lookup_table_account: None,
            throttled_nodes: HashMap::new(),
            triggering_nodes: HashMap::new(),
            user_stats_map: None,
            use_burst_cu_limit: false,
            watchdog_timer_last_pat_time: Instant::now(),
        }
    }

    fn record_evicted_tx_sig(&self) {
        todo!()
    }

    fn initialize_metrics(&self) {
        todo!()
    }

    pub async fn base_init(&mut self) {
        let filler_sol_balance = self
            .drift_client
            .backend
            .rpc_client
            .get_balance(self.drift_client.wallet().authority())
            .await
            .expect("get sol balance");
        self.has_enough_sol_to_fill = filler_sol_balance as f64 >= self.min_gas_balance_to_fill;
        log::info!(
            "{}: has_enoght_sol_to_fill: {}, balance: {filler_sol_balance}",
            self.name,
            self.has_enough_sol_to_fill
        );

        let start_init_user_stats_map = Instant::now();
        info!("Initializing user_stats_map");

        // TODO: temp value Duration
        let user_stats_loader = BulkAccountLoader::new(
            self.drift_client.backend.rpc_client.clone(),
            CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            },
            Duration::from_secs(60),
        );
        let user_stats_map = UserStatsMap::new(self.drift_client.clone(), Some(user_stats_loader));

        log::info!(
            "Initialized user_stats_map: {}, took: {}ms",
            user_stats_map.size(),
            start_init_user_stats_map.elapsed().as_millis()
        );

        self.user_stats_map = Some(user_stats_map);

        self.clock_subscriber
            .subscribe()
            .await
            .expect("subscribe clock");

        self.lookup_table_account = Some(self.drift_client.fetch_market_lookup_table_account());
    }

    pub async fn init(&mut self) {
        self.base_init().await;
        let drift_client = self.drift_client.clone();
        let user_map = self.user_map.clone().unwrap();
        let slot_subscriber = self.slot_subscriber.clone();

        let dlob_subscriber = DLOBSubscriber::new(DLOBSubscriptionConfig {
            drift_client,
            dlob_source: DlobSource::UserMap(user_map),
            slot_source: SlotSource::SlotSubscriber(slot_subscriber),
            update_frequency: Duration::from_millis((self.polling_interval_ms - 500) as u64),
        });
        self.dlob_subscriber = Some(dlob_subscriber);

        if let Some(dlob_subscriber) = &self.dlob_subscriber {
            dlob_subscriber.subscribe().await.unwrap();
        }

        log::info!("[{}]: started", self.name);
    }

    pub async fn reset(&mut self) {
        if let Some(dlob_sub) = &mut self.dlob_subscriber {
            dlob_sub.unsubscribe().await;
        }
        if let Some(user_map) = &mut self.user_map {
            user_map.unsubscribe().await.expect("unsubscribe usermap");
        }
    }

    pub async fn start_interval_loop(&mut self) {
        self.try_fill().await;
        self.settle_pnls().await;
        self.confirm_pending_tx_sigs().await;

        log::info!(
            "{} Bot started! (websocket: {})",
            self.name,
            self.bulk_account_loader.is_none()
        );
    }

    fn record_jito_bundle_stats() {
        todo!()
    }

    async fn confirm_pending_tx_sigs(&mut self) {
        let next_time_can_run =
            self.confirm_loop_rate_limit_ts + Duration::from_secs(CONFIRM_TX_RATE_LIMIT_BACKOFF_MS);
        let now = Instant::now();
        if now < next_time_can_run {
            log::warn!(
                "Skipping confirm loop due to rate limit, next run in {} ms",
                (next_time_can_run - now).as_millis()
            );
            return;
        }
        if self.confirm_loop_running {
            return;
        }
        self.confirm_loop_running = true;

        log::info!(
            "Confirming tx sigs: {}",
            self.pending_tx_sigs_toconfirm.len()
        );
        let start = Instant::now();
        let pending_tx_sigs_toconfirm = self.pending_tx_sigs_toconfirm.clone();
        let tx_entries: Vec<(&Signature, &PendingTxSigsToconfirm)> =
            pending_tx_sigs_toconfirm.iter().collect();
        for i in (0..tx_entries.len()).step_by(TX_CONFIRMATION_BATCH_SIZE) {
            let tx_sigs_batch = &tx_entries[i..i + TX_CONFIRMATION_BATCH_SIZE];
            let sigs: Vec<Signature> = tx_sigs_batch
                .into_iter()
                .map(|(sig, _pending_tx)| **sig)
                .collect();

            let fetches = sigs.iter().map(|sig| {
                self.tx_confirmation_connection
                    .get_transaction(sig, UiTransactionEncoding::Json)
            });
            let txs = futures_util::future::join_all(fetches).await;

            for j in 0..txs.len() {
                let tx_resp = &txs[j];
                let tx_confirmation_info = tx_sigs_batch[j];
                let tx_sig = tx_confirmation_info.0;
                let tx_age = tx_confirmation_info.1.ts - Instant::now();
                let node_filled = &tx_confirmation_info.1.node_filled;
                let tx_type = &tx_confirmation_info.1.tx_type;
                let fill_tx_id = tx_confirmation_info.1.fill_tx_id;

                match tx_resp {
                    Ok(tx) => {
                        log::info!("Tx landed (fill_tx_id: {fill_tx_id}) (tx_type: {tx_type:?}): {tx_sig}, tx age: {} s", tx_age.as_secs());
                        self.pending_tx_sigs_toconfirm.pop(tx_sig);

                        if matches!(tx_type, TxType::Fill) {
                            if let Some(meta) = &tx.transaction.meta {
                                if let OptionSerializer::Some(msgs) = &meta.log_messages {
                                    let _result =
                                        self.handle_transaction_logs(&node_filled, msgs).await;
                                }
                            }
                        }

                        log::info!(
                            "Confirming tx sigs took: {} ms",
                            start.elapsed().as_millis()
                        );
                    }
                    Err(e) => {
                        if e.to_string().contains("429") {
                            log::info!("Confirming tx loop rate limited: {}", e.to_string());
                            self.confirm_loop_rate_limit_ts = Instant::now();
                        }

                        log::info!("Tx not found, (fill_tx_id: {fill_tx_id}) (tx_type: {tx_type:?}: {tx_sig}, tx age: {} s", tx_age.as_secs());
                        if tx_age.as_millis() > TX_TIMEOUT_THRESHOLD_MS {
                            self.pending_tx_sigs_toconfirm.pop(tx_sig);
                        }
                    }
                }
            }

            self.confirm_loop_running = false;
        }
    }

    pub fn health_check(&self) -> bool {
        let healthy = false;

        healthy
    }

    async fn get_user_account_and_slot_from_map(&self, key: Pubkey) -> Option<(User, u64)> {
        if let Some(user_map) = &self.user_map {
            let (user, slot) = user_map
                .must_get_with_slot(key)
                .await
                .expect("must get with user and slot");
            return Some((user, slot));
        }

        None
    }

    async fn get_dlob(&self) -> Option<DLOB> {
        if let Some(dlob_sub) = &self.dlob_subscriber {
            return Some(dlob_sub.get_dlob().await);
        }

        None
    }

    fn get_max_slot(&self) -> u64 {
        let slot_x = self.slot_subscriber.get_slot();
        let slot_y = match &self.user_map {
            Some(map) => map.get_latest_slot(),
            None => 0,
        };

        std::cmp::max(slot_x, slot_y)
    }

    fn log_slots(&self) {
        let slot = match &self.user_map {
            Some(map) => map.get_latest_slot(),
            None => 0,
        };
        log::info!(
            "slot_subscriber slot: {}, user_map slot: {}",
            self.slot_subscriber.get_slot(),
            slot
        );
    }

    /// Return `nodes_to_fill`, `nodes_to_trigger`
    async fn get_perp_nodes_for_market(
        &self,
        market: PerpMarket,
        dlob: &mut DLOB,
    ) -> Result<(Vec<NodeToFill>, Vec<Node>), String> {
        let market_index = market.market_index;

        let oracle = self
            .drift_client
            .get_oracle_price_data_and_slot_for_perp_market(market_index);
        if let Some(oracle) = oracle {
            let v_ask = calculate_ask_price(&market, &oracle.data).map_err(|e| e.to_string())?;
            let v_bid = calculate_bid_price(&market, &oracle.data).map_err(|e| e.to_string())?;
            let fill_slot = self.get_max_slot();

            let state_account = self.drift_client.get_state_account();
            let state = state_account.read().expect("read state account");
            let perp_market = self
                .drift_client
                .get_perp_market_account(market_index)
                .expect("get perp market_account");

            let nodes_to_fill = dlob
                .find_nodes_to_fill(
                    market_index,
                    v_bid,
                    v_ask,
                    fill_slot,
                    self.clock_subscriber.get_unix_ts().await - EXPIRE_ORDER_BUFFER_SEC,
                    MarketType::Perp,
                    &oracle.data,
                    &state,
                    &MarketAccount::PerpMarket(Box::new(perp_market)),
                )
                .map_err(|e| e.to_string())?;

            let nodes_to_trigger = dlob.find_nodes_to_trigger(
                market_index,
                oracle.data.price as u64,
                MarketType::Perp,
                self.drift_client.get_state_account(),
            );

            return Ok((nodes_to_fill, nodes_to_trigger));
        }

        Err(String::from("Could not find oracle"))
    }

    /// Check if the node is still throttled, if not, clears it from the throttled_nodes map
    fn is_throttled_node_still_throttled(&mut self, throttle_key: String) -> bool {
        if let Some(last_fill_attempt) = self.throttled_nodes.get(&throttle_key.to_string()) {
            let duration = Duration::new(FILL_ORDER_THROTTLE_BACKOFF, 0);
            if *last_fill_attempt + duration > Instant::now() {
                return true;
            } else {
                self.clear_throttled_node(throttle_key);
                return false;
            }
        }

        false
    }

    fn is_dlob_node_throttled(&mut self, dlob_node: &Node) -> bool {
        // first check if the user_account itself is throttled
        let user_account_pubkey = dlob_node.get_user_account();
        if self
            .throttled_nodes
            .contains_key(&user_account_pubkey.to_string())
        {
            if self.is_throttled_node_still_throttled(user_account_pubkey.to_string()) {
                return true;
            } else {
                return false;
            }
        }

        // then check if the specific order is throttled
        let order_sig = get_fill_signature_from_user_account_and_orader_id(
            user_account_pubkey,
            dlob_node.get_order().order_id,
        );
        if self.throttled_nodes.contains_key(&order_sig) {
            if self.is_throttled_node_still_throttled(order_sig) {
                return true;
            } else {
                return false;
            }
        }

        false
    }

    fn clear_throttled_node(&mut self, sig: String) {
        self.throttled_nodes.remove(&sig);
    }

    fn set_throttled_node(&mut self, sig: &str) {
        self.throttled_nodes.insert(sig.to_string(), Instant::now());
    }

    fn prune_throttled_node(&mut self) {
        if self.throttled_nodes.len() > THROTTLED_NODE_SIZE_TO_PRUNE {
            let now = Instant::now();
            let duration_threshold = Duration::new(2_u64 * FILL_ORDER_THROTTLE_BACKOFF, 0);

            self.throttled_nodes
                .retain(|_, v| *v + duration_threshold <= now)
        }
    }

    async fn filter_fillable_nodes(&mut self, node_to_fill: &NodeToFill) -> bool {
        let node = node_to_fill.get_node();

        if node.is_vamm_node() {
            log::warn!(
                "filtered out a vAMM node on market {} for user {}-{}",
                node.get_order().market_index,
                node.get_user_account(),
                node.get_order().order_id
            );
            return false;
        }

        // if (nodeToFill.node.haveFilled) {
        // 	logger.warn(
        // 		`filtered out filled node on market ${nodeToFill.node.order.marketIndex} for user ${nodeToFill.node.userAccount}-${nodeToFill.node.order.orderId}`
        // 	);
        // 	return false;
        // }

        let now = Instant::now();
        let node_to_fill_signature = get_node_to_fill_signature(node_to_fill);
        if self.filling_nodes.contains_key(&node_to_fill_signature) {
            if let Some(time_started_to_fill_node) = self.filling_nodes.get(&node_to_fill_signature)
            {
                let duration = Duration::new(FILL_ORDER_THROTTLE_BACKOFF, 0);
                if *time_started_to_fill_node + duration > now {
                    // still cooling down on this node, filter it out
                    return false;
                }
            }
        }

        // expired orders that we previously tried to fill
        if self.expired_nodes_set.contains(&node_to_fill_signature) {
            return false;
        }

        // check if taker node is throttled
        if self.is_dlob_node_throttled(&node_to_fill.get_node()) {
            return false;
        }

        let user_account = node.get_user_account();
        let order = node.get_order();
        let market_index = order.market_index;
        let oracle = self
            .drift_client
            .get_oracle_price_data_and_slot_for_perp_market(market_index);

        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        if is_order_expired(order, since_the_epoch.as_secs() as i64, Some(true), None) {
            if matches!(order.order_type, OrderType::Limit) {
                // do not try to fill (expire) limit orders b/c they will auto expire when filled
                // against
                // or the user places a new order
                return false;
            }

            return true;
        }

        if let Some(oracle_price_data) = oracle {
            let market_info = self
                .drift_client
                .get_perp_market_info(market_index)
                .await
                .expect("find perp market info");
            let state_account = self.drift_client.get_state_account();
            let state = state_account.read().expect("read state account");
            if node_to_fill.get_maker_nodes().is_empty()
                && matches!(order.market_type, MarketType::Perp)
                && is_fillable_by_vamm(
                    order,
                    market_info,
                    &oracle_price_data.data,
                    self.get_max_slot(),
                    since_the_epoch.as_secs() as i64,
                    state.min_perp_auction_duration,
                )
                .expect("is fillable by vamm")
            {
                log::warn!(
                    "filtered out unfillable node on market {} for user {}-{}",
                    market_index,
                    user_account,
                    order.order_id
                );
                log::warn!(
                    " . no maker node: {}",
                    node_to_fill.get_maker_nodes().is_empty()
                );
                log::warn!(
                    " . is perp: {}",
                    matches!(order.market_type, MarketType::Perp)
                );
                return false;
            }

            let perp_market = self
                .drift_client
                .get_perp_market_info(market_index)
                .await
                .expect("find perp market info");

            // if making with vAMM, ensure valid oracle
            if node_to_fill.get_maker_nodes().is_empty()
                && !matches!(perp_market.amm.oracle_source, OracleSource::Prelaunch)
            {
                let oracle_is_valid = is_oracle_valid(
                    &perp_market,
                    &oracle_price_data.data,
                    &state.oracle_guard_rails,
                    self.get_max_slot(),
                );

                if !oracle_is_valid {
                    log::error!(
                        "Oracle is not valid for market {market_index}, skipping fill with vAMM"
                    );
                    return false;
                }
            }
        }

        true
    }

    fn filter_triggerable_nodes(&self, node_to_trigger: &Node) -> bool {
        if matches!(node_to_trigger.get_node_type(), NodeType::Trigger) {
            return false;
        }

        let now = Instant::now();
        let node_to_fill_sig = get_node_to_trigger_signature(node_to_trigger);
        if let Some(time_started_to_trigger_node) = self.triggering_nodes.get(&node_to_fill_sig) {
            let duration = Duration::new(TRIGGER_ORDER_COOLDOWN_MS, 0);
            if *time_started_to_trigger_node + duration > now {
                return false;
            }
        }

        true
    }

    /// Return `maker_info`, `taker_user_pubkey`, `taker_user`, `taker_user_slot`, `referrer_info`,
    /// `market_type`
    async fn get_node_fill_info(
        &mut self,
        node_to_fill: &NodeToFill,
    ) -> Option<(
        Vec<(u64, MakerInfo)>,
        Pubkey,
        User,
        u64,
        Option<ReferrerInfo>,
        MarketType,
    )> {
        let mut maker_infos = Vec::new();

        if !node_to_fill.get_maker_nodes().is_empty() {
            let mut maker_nodes_map = HashMap::new();
            for maker_node in node_to_fill.get_maker_nodes() {
                if self.is_dlob_node_throttled(maker_node) {
                    continue;
                }

                let user_account = maker_node.get_user_account();
                // if maker_node.get_user_account()

                maker_nodes_map
                    .entry(user_account)
                    .and_modify(|dlob_nodes: &mut Vec<Node>| dlob_nodes.push(*maker_node))
                    .or_insert(vec![*maker_node]);
            }

            if maker_nodes_map.len() > MAX_MAKERS_PER_FILL {
                log::info!("selecting from {} makers", maker_nodes_map.len());
                maker_nodes_map = select_makers(&mut maker_nodes_map);
                // log::info!("selected: {}", maker_nodes_map.keys)
            }

            for (maker_account, maker_nodes) in maker_nodes_map {
                let maker_node = maker_nodes[0];

                if let Some((maker_user_account, slot)) =
                    self.get_user_account_and_slot_from_map(maker_account).await
                {
                    let maker_authority = maker_user_account.authority;

                    if let Some(ref mut user_stats_map) = self.user_stats_map {
                        let user_stats = user_stats_map
                            .must_get(&maker_authority)
                            .await
                            .expect("must get userstats");
                        if let Some(user_stats) = user_stats {
                            let maker_user_stats = user_stats.user_stats_account_pubkey;

                            maker_infos.push((
                                slot,
                                MakerInfo::new(
                                    maker_account,
                                    maker_user_stats,
                                    maker_user_account,
                                    Some(*maker_node.get_order()),
                                ),
                            ));
                        }
                    }
                }
            }
        }

        let taker_user_pubkey = node_to_fill.get_node().get_user_account();
        if let Some((taker_user_account, taker_user_account_slot)) = self
            .get_user_account_and_slot_from_map(taker_user_pubkey)
            .await
        {
            if let Some(ref mut user_stats_map) = self.user_stats_map {
                let user_stats = user_stats_map
                    .must_get(&taker_user_account.authority)
                    .await
                    .expect("must get userstats");
                if let Some(user_stats) = user_stats {
                    let referrer_info = user_stats.get_referrer_info().expect("get referrer info");

                    return Some((
                        maker_infos,
                        taker_user_pubkey,
                        taker_user_account,
                        taker_user_account_slot,
                        referrer_info,
                        node_to_fill.get_node().get_order().market_type,
                    ));
                }
            }
        }

        None
    }

    // Returns the number of bytes occupied by this array if it were serialized in compact-u16-format.
    // NOTE: assumes each element of the array is 1 byte (not sure if this holds?)
    //
    // https://docs.solana.com/developing/programming-model/transactions#compact-u16-format
    //
    // https://stackoverflow.com/a/69951832
    //  hex     |  compact-u16
    //  --------+------------
    //  0x0000  |  [0x00]
    //  0x0001  |  [0x01]
    //  0x007f  |  [0x7f]
    //  0x0080  |  [0x80 0x01]
    //  0x3fff  |  [0xff 0x7f]
    //  0x4000  |  [0x80 0x80 0x01]
    //  0xc000  |  [0x80 0x80 0x03]
    //  0xffff  |  [0xff 0xff 0x03])
    //
    fn calc_compact_u16_encoded_size<A>(&self, array: &[A], elem_size: Option<usize>) -> usize {
        let elem_size = elem_size.unwrap_or(1);

        if array.len() > 0x3fff {
            3 + array.len() * elem_size
        } else if array.len() > 0x7f {
            2 + array.len() * elem_size
        } else {
            let array_len = array.len();
            let product = array_len * elem_size;
            let safe_product = if product == 0 { 1 } else { product };
            1 + safe_product
        }
    }

    async fn build_tx_with_maker_infos(
        &mut self,
        makers: &[MakerInfo],
        param_ixs: &[Instruction],
        node_to_fill: &NodeToFill,
        taker_user: &User,
        referrer_info: &Option<ReferrerInfo>,
    ) -> SimulateAndGetTxWithCUsResponse {
        let user_account_pubkey = self.drift_client.wallet().authority();
        let mut builder = self
            .drift_client
            .init_tx(&user_account_pubkey, false)
            .expect("build tx")
            .fill_perp_order(
                *user_account_pubkey,
                taker_user,
                node_to_fill.get_node().get_order(),
                makers,
                referrer_info,
            );

        let sig = get_node_to_fill_signature(node_to_fill);
        self.filling_nodes.insert(sig, Instant::now());

        if self.revert_on_failure.is_some() {
            builder = builder.revert_fill(*user_account_pubkey);
        }

        let builder_ixs: Vec<Instruction> = builder
            .instructions()
            .into_iter()
            .map(|ix| ix.clone())
            .collect();
        let mut ixs = Vec::new();
        for param_ix in param_ixs.into_iter() {
            ixs.push(param_ix.clone());
        }

        for builder_ix in builder_ixs {
            ixs.push(builder_ix);
        }

        let recent_blockhash = self
            .drift_client
            .backend
            .rpc_client
            .get_latest_blockhash()
            .await
            .expect("get recent blockhash");

        let mut params = SimulateAndGetTxWithCUsParams {
            connection: self.drift_client.backend.rpc_client.clone(),
            payer: self.drift_client.wallet.signer.clone(),
            lookup_table_accounts: vec![self
                .lookup_table_account
                .clone()
                .expect("lookup table account")
                .clone()],
            ixs: ixs.into(),
            cu_limit_multiplier: Some(SIM_CU_ESTIMATE_MULTIPLIER),
            do_simulation: Some(true),
            recent_blockhash: Some(recent_blockhash),
            dump_tx: None,
        };

        let sim_res = simulate_and_get_tx_with_cus(&mut params)
            .await
            .expect("simulate");

        sim_res
    }

    // Instruction are made of 3 parts:
    // - index of accounts where program_id resides (1 byte)
    // - affected accounts (compact-u16-format byte array)
    // - raw instruction data (compact-u16-format byte array)
    fn calc_ix_encoded_size(&self, ixs: &[Instruction]) -> usize {
        let mut sum = 0;

        for ix in ixs {
            sum += 1
                + self.calc_compact_u16_encoded_size(&[ix.accounts.len()], Some(1))
                + self.calc_compact_u16_encoded_size(&[ix.data.len()], Some(1));
        }

        sum
    }

    /// Iterates through a tx's logs and handles it appropriately (e.g. throttling users, updating metrics, etc.)
    ///
    /// Returns `filled_nodes`, `exceeded_cus`
    async fn handle_transaction_logs(
        &mut self,
        nodes_filled: &[NodeToFill],
        logs: &[String],
    ) -> (usize, bool) {
        if logs.is_empty() {
            return (0, false);
        }

        let mut in_fill_ix = false;
        let mut error_this_fill_ix = false;
        let mut ix_idx: usize = 0; // skip ComputeBudgeProgram
        let mut success_count = 0;
        let mut bursted_cu = false;
        for log in logs {
            if log.is_empty() {
                log::error!("log is null");
                continue;
            }

            if log.contains("exceeded maximum number of instructions allowed") {
                // temporary burst CU limit
                log::warn!("Using bursted CU limit");
                self.use_burst_cu_limit = true;
                self.fill_tx_since_burst_cu = 0;
                bursted_cu = true;
                continue;
            }

            if is_end_ix_log(constants::PROGRAM_ID, log) {
                if in_fill_ix && !error_this_fill_ix {
                    success_count += 1;
                }

                in_fill_ix = false;
                error_this_fill_ix = false;
                continue;
            }

            if is_ix_log(log) {
                if is_fill_ix_log(log) {
                    in_fill_ix = true;
                    error_this_fill_ix = false;
                    ix_idx += 1;
                } else {
                    in_fill_ix = false;
                }

                continue;
            }

            if !in_fill_ix {
                // this is not a log for a fill instruction
                continue;
            }

            // try to handle the log line
            if let Some(_order_id) = is_order_does_not_exist_log(log) {
                if let Some(filled_node) = nodes_filled.get(ix_idx) {
                    let now = SystemTime::now();
                    let since_the_epoch =
                        now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                    let is_expired = is_order_expired(
                        filled_node.get_node().get_order(),
                        since_the_epoch.as_secs() as i64,
                        Some(true),
                        None,
                    );

                    log::error!("assoc node (ix_idx: {ix_idx}): {}, {}; does not exist (filled by someone else); {log}, expired: {is_expired}, order_ts: {}, now: {}", filled_node.get_node().get_user_account(), filled_node.get_node().get_order().order_id, filled_node.get_node().get_order().max_ts, since_the_epoch.as_secs());

                    if is_expired {
                        let sig = get_node_to_fill_signature(filled_node);
                        self.expired_nodes_set.put(sig, true);
                    }
                }

                error_this_fill_ix = true;
                continue;
            }

            if let Some(margin) = is_maker_breached_maintainance_margin_log(log) {
                log::error!("Throttling maker breached maintainance margin: {margin}");
                self.set_throttled_node(&margin);
                let user_pub = Pubkey::from_str(&margin).unwrap();
                if let Some((user_account, _slot)) =
                    self.get_user_account_and_slot_from_map(user_pub).await
                {
                    let msg = self
                        .drift_client
                        .init_tx(self.drift_client.wallet().authority(), false)
                        .unwrap()
                        .force_cancel_orders(None, user_pub, &user_account)
                        .legacy()
                        .build();

                    match self.drift_client.sign_and_send(msg, false).await {
                        Ok(sig) => {
                            log::info!("force_cancel_orders for makers due to breach of maintainance margin. Tx: {sig}");
                        }
                        Err(e) => {
                            log::error!("{e}");
                            log::error!("Failed to send force_calcel_order Tx for maker ({margin}) breach margin (error above)");
                        }
                    }
                }

                // error_this_fill_ix = true;
                break;
            }

            if let Some(filled_node) = nodes_filled.get(ix_idx) {
                if is_taker_breached_maintainance_margin_log(log) {
                    let taker_node_sig = filled_node.get_node().get_user_account();
                    log::error!("taker breach maint. margin, assoc node (ix_idx: {ix_idx}): {}, {}; (throttling {taker_node_sig} and force cancelling orders); {log}", filled_node.get_node().get_user_account(), filled_node.get_node().get_order().order_id);
                    self.set_throttled_node(&taker_node_sig.to_string());
                    error_this_fill_ix = true;

                    let user_pub = filled_node.get_node().get_user_account();
                    if let Some((user_account, _slot)) =
                        self.get_user_account_and_slot_from_map(user_pub).await
                    {
                        let msg = self
                            .drift_client
                            .init_tx(self.drift_client.wallet().authority(), false)
                            .unwrap()
                            .force_cancel_orders(None, user_pub, &user_account)
                            .legacy()
                            .build();

                        match self.drift_client.sign_and_send(msg, false).await {
                            Ok(sig) => {
                                log::info!("force_cancel_orders for user {user_pub} due to breach of maintainance margin. Tx: {sig}");
                            }
                            Err(e) => {
                                log::error!("{e}");
                                log::error!("Failed to send force_calcel_order Tx for taker ({user_pub} - {}) breach maint. margin (error above)", filled_node.get_node().get_order().order_id);
                            }
                        }
                    }

                    continue;
                }
            }

            if let (Some(order_id), Some(user_account)) = is_err_filling_log(log) {
                let extract_sig =
                    get_fill_signature_from_user_account_and_orader_id(user_account, order_id);
                self.set_throttled_node(&extract_sig);

                if let Some(filled_node) = nodes_filled.get(ix_idx) {
                    let assoc_node_sig = get_node_to_fill_signature(filled_node);
                    log::warn!("Throttling node due to fill error. extracted_sig: {extract_sig}, assoc_node_sig: {assoc_node_sig}, assoc_node_idx: {ix_idx}");
                    error_this_fill_ix = true;
                    continue;
                }
            }

            if is_err_stale_oracle(log) {
                log::error!("Stale oracle error: {log}");
                error_this_fill_ix = true;
                continue;
            }
        }

        if !bursted_cu {
            if self.fill_tx_since_burst_cu > TX_COUNT_COOLDOWN_ON_BURST {
                self.use_burst_cu_limit = false;
            }
            self.fill_tx_since_burst_cu += 1;
        }

        if !logs.is_empty() {
            if let Some(last) = logs.last() {
                if last.contains("exceeded CUs meter at BPF instruction") {
                    return (success_count, true);
                }
            }
        }

        (success_count, false)
    }

    /// Queues up the tx_sig to be confirmed in a slower loop, and have tx logs handled
    fn register_tx_sig_to_confirm(
        &mut self,
        tx_sig: Signature,
        now: Instant,
        node_filled: &[NodeToFill],
        fill_tx_id: u16,
        tx_type: TxType,
    ) {
        self.pending_tx_sigs_toconfirm.put(
            tx_sig,
            PendingTxSigsToconfirm::new(now, node_filled, fill_tx_id, tx_type),
        );
    }

    fn remove_filling_nodes(&mut self, nodes: &[NodeToFill]) {
        for node in nodes {
            self.filling_nodes.remove(&get_node_to_fill_signature(node));
        }
    }

    async fn send_tx_through_jito(
        &self,
        tx: &VersionedTransaction,
        metadata: &str,
        tx_sig: Option<Signature>,
    ) {
        match &self.bundle_sender {
            Some(sender) => {
                if matches!(
                    sender.strategy,
                    JitoStrategy::JitoOnly | JitoStrategy::Hybrid
                ) {
                    let slots_until_next_leader = sender.slots_until_next_leader();
                    if let Some(_leader) = slots_until_next_leader {
                        sender
                            .send_transaction(tx, Some(format!("(fill_tx_id: {metadata})")), tx_sig)
                            .await;
                    }
                }
            }
            None => {
                log::error!("Called send_tx_through_jito without jito property enabled");
                return;
            }
        }
    }

    async fn send_fill_tx_and_parse_logs(
        &mut self,
        fill_tx_id: u16,
        nodes_sent: &[NodeToFill],
        tx: VersionedTransaction,
        build_for_bundle: bool,
    ) {
        if let Some(look_up_table_account) = &self.lookup_table_account {
            let (_est_tx_size, _account_metas, write_accs, tx_accounts) =
                get_transaction_account_metas(&tx, &[look_up_table_account]);

            let tx_start = Instant::now();
            let tx_sig = tx.signatures[0];

            if build_for_bundle {
                self.send_tx_through_jito(&tx, &format!("{fill_tx_id}"), Some(tx_sig))
                    .await;
                self.remove_filling_nodes(nodes_sent);
            } else if self.can_send_outside_jito() {
                match self.drift_client.sign_and_send(tx.message, false).await {
                    Ok(resp) => {
                        log::info!(
                            "sent tx: {resp}, took: {}ms (fill_tx_id: {fill_tx_id}",
                            tx_start.elapsed().as_millis()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to send packed tx tx_account_keys: {tx_accounts} ({write_accs} writeable) (fill_tx_id: {fill_tx_id}), error: {e}");
                    }
                }
            }

            self.register_tx_sig_to_confirm(
                tx_sig,
                Instant::now(),
                nodes_sent,
                fill_tx_id,
                TxType::Fill,
            );
        }
    }

    async fn fill_multi_maker_perp_nodes(
        &mut self,
        fill_tx_id: u16,
        node_to_fill: &NodeToFill,
        build_for_bundle: bool,
    ) -> Result<bool, String> {
        let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)];
        if !build_for_bundle {
            ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
                self.priority_fee_subscriber.get_custom_strategy_result() as u64,
            ));
        }

        if let Some((
            maker_infos,
            _taker_user_pubkey,
            taker_user,
            taker_user_slot,
            referrer_info,
            market_type,
        )) = self.get_node_fill_info(node_to_fill).await
        {
            if MarketType::Perp != market_type {
                return Err(String::from("expected perp market type"));
            }

            let _user = self.drift_client.get_user(None);
            let mut maker_infos_to_use: Vec<MakerInfo> = maker_infos
                .into_iter()
                .map(|(_slot, maker_info)| maker_info)
                .collect();

            let mut sim_res = self
                .build_tx_with_maker_infos(
                    &maker_infos_to_use,
                    &ixs,
                    node_to_fill,
                    &taker_user,
                    &referrer_info,
                )
                .await;
            let mut tx_accounts = sim_res.tx.message.static_account_keys().len();
            let attempt = 0;
            while tx_accounts > MAX_ACCOUNTS_PER_TX && maker_infos_to_use.len() > 0 {
                log::info!("(fill_tx_id: {fill_tx_id} attempt {attempt}) Too many accounts, remove 1 and try again (had {} maker and {tx_accounts} accounts)", maker_infos_to_use.len());
                maker_infos_to_use = maker_infos_to_use[0..maker_infos_to_use.len() - 1].to_vec();
                sim_res = self
                    .build_tx_with_maker_infos(
                        &maker_infos_to_use,
                        &ixs,
                        node_to_fill,
                        &taker_user,
                        &referrer_info,
                    )
                    .await;
                tx_accounts = sim_res.tx.message.static_account_keys().len();
            }

            if maker_infos_to_use.is_empty() {
                log::error!("No maker_infos left to use for multi maker perp node (fill_tx_id: {fill_tx_id}");
                return Ok(true);
            }

            match sim_res.sim_error {
                Some(err) => {
                    log::error!("Error simulating multi maker perp node (fill_ix_id: {fill_tx_id}: {:?}\nTaker slot: {taker_user_slot}\n", err);

                    if let Some(logs) = sim_res.sim_tx_logs {
                        let (_filled_nodes, _exceeded_cus) = self
                            .handle_transaction_logs(&[node_to_fill.clone()], &logs)
                            .await;
                    }
                }
                None => {
                    if self.dry_run {
                        log::info!("dry run, not sending tx (fill_tx_id: {fill_tx_id})");
                    } else {
                        if self.has_enough_sol_to_fill {
                            self.send_fill_tx_and_parse_logs(
                                fill_tx_id,
                                &[node_to_fill.clone()],
                                sim_res.tx,
                                build_for_bundle,
                            )
                            .await;
                        } else {
                            log::info!("not sending tx because we don't have enough SOL to fill (fill_tx_id: {fill_tx_id}");
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// It's difficult to estimate CU cost of multi maker ix, so we'll just send it in its own transaction
    async fn try_fill_multi_maker_perp_nodes(
        &mut self,
        node_to_fill: &NodeToFill,
        build_for_bundle: bool,
    ) {
        let fill_tx_id = self.fill_tx_id;
        self.fill_tx_id += 1;

        let mut node_with_market_set = node_to_fill.clone();
        while !self
            .fill_multi_maker_perp_nodes(fill_tx_id, &node_with_market_set, build_for_bundle)
            .await
            .expect("fill multi maker perp nodes")
        {
            let mut maker_nodes: Vec<Node> = node_with_market_set.get_maker_nodes().to_vec();

            let mut rng = thread_rng();
            maker_nodes.shuffle(&mut rng);

            let midpoint = (maker_nodes.len() as f64 / 2.0).ceil() as usize;

            let new_maker_set = maker_nodes[0..midpoint].to_vec();
            if new_maker_set.is_empty() {
                log::error!(
                    "No makers left to use for multi maker perp node (fill_tx_id: {fill_tx_id}"
                );
                return;
            }
            node_with_market_set = NodeToFill::new(node_with_market_set.get_node(), new_maker_set);
        }
    }

    async fn try_fill_perp_nodes(
        &mut self,
        nodes_to_fill: &[NodeToFill],
        build_for_bundle: bool,
    ) -> usize {
        let mut nodes_sent = 0;
        let mut market_node_map = HashMap::new();

        for node_to_fill in nodes_to_fill {
            let market_index = node_to_fill.get_node().get_order().market_index;
            market_node_map
                .entry(market_index)
                .and_modify(|nodes: &mut Vec<NodeToFill>| nodes.push(node_to_fill.clone()))
                .or_insert(Vec::new());
        }

        for nodes_to_fill_for_market in market_node_map.values() {
            let sent = self
                .try_fill_perp_nodes_for_market(nodes_to_fill_for_market, build_for_bundle)
                .await
                .expect("try fill");
            nodes_sent += sent;
        }

        nodes_sent
    }

    async fn try_fill_perp_nodes_for_market(
        &mut self,
        nodes_to_fill: &[NodeToFill],
        build_for_bundle: bool,
    ) -> Result<usize, String> {
        let drift_client = self.drift_client.clone();
        let user_account_pubkey = drift_client.wallet().authority();
        let mut builder = drift_client
            .init_tx(&user_account_pubkey, false)
            .expect("build tx");
        let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)];
        if !build_for_bundle {
            ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
                self.priority_fee_subscriber.get_custom_strategy_result() as u64,
            ));
        }

        //
        // At all times, the running Tx size is:
        // - signatures (compact-u16 array, 64 bytes per elem)
        // - message header (3 bytes)
        // - affected accounts (compact-u16 array, 32 bytes per elem)
        // - previous block hash (32 bytes)
        // - message instructions (
        //		- progamIdIdx (1 byte)
        //		- accountsIdx (compact-u16, 1 byte per elem)
        //   	- instruction data (compact-u16, 1 byte per elem)
        //
        let mut running_tx_size = 0;
        let mut running_cu_used = 0;

        let mut unique_accounts = HashSet::new();
        unique_accounts.insert(*self.drift_client.wallet().authority());

        let compute_budget_ix = &ixs[0];
        for key in compute_budget_ix.accounts.iter() {
            unique_accounts.insert(key.pubkey);
        }
        unique_accounts.insert(compute_budget_ix.program_id);

        // initialize the barebones transactions
        // signatures
        running_tx_size += self.calc_compact_u16_encoded_size(&[1], Some(64));
        // msssage header
        running_tx_size += 3;
        // accounts
        running_tx_size += self.calc_compact_u16_encoded_size(&[unique_accounts.len()], Some(32));
        // blockhash
        running_tx_size += 32;
        running_tx_size += self.calc_ix_encoded_size(&[compute_budget_ix.clone()]);

        let mut nodes_sent: Vec<_> = Vec::new();
        let mut idx_used = 0;
        let starting_ixs_size = ixs.len();
        let fill_tx_id = self.fill_tx_id;
        self.fill_tx_id += 1;

        for node_to_fill in nodes_to_fill.iter() {
            // do multi maker fills in a separate tx since they're larger
            if !node_to_fill.get_maker_nodes().is_empty() {
                self.try_fill_multi_maker_perp_nodes(node_to_fill, build_for_bundle)
                    .await;
                nodes_sent.push(node_to_fill);
                continue;
            }

            // otherwise pack fill ixs untis est. tx size or CU limit is hit
            if let Some((
                maker_infos,
                _taker_user_pubkey,
                taker_user,
                _taker_user_slot,
                referrer_info,
                market_type,
            )) = self.get_node_fill_info(node_to_fill).await
            {
                // log_message_fo_node_to_fill
                log::info!("");

                if !matches!(market_type, MarketType::Perp) {
                    return Err(String::from("expected perp market type"));
                }

                let maker_info: Vec<MakerInfo> =
                    maker_infos.into_iter().map(|(_, info)| info).collect();
                builder = builder.fill_perp_order(
                    *user_account_pubkey,
                    &taker_user,
                    node_to_fill.get_node().get_order(),
                    &maker_info,
                    &referrer_info,
                );

                let instructions = builder.instructions();
                if instructions.is_empty() {
                    log::error!("failed to generate an ix");
                    break;
                }

                let sig = get_node_to_fill_signature(node_to_fill);
                self.filling_nodes.insert(sig, Instant::now());

                // first estimate new tx size with this additional ix and new accounts
                let mut ix_keys = Vec::new();
                for ix in ixs.clone() {
                    ix_keys.extend(ix.accounts);
                }
                let mut new_accounts: Vec<AccountMeta> = Vec::new();
                for ix_key in ix_keys {
                    if !unique_accounts.contains(&ix_key.pubkey) {
                        new_accounts.push(ix_key);
                    }
                }
                let new_ix_cost = self.calc_ix_encoded_size(instructions);
                let additional_accounts_cost = if new_accounts.is_empty() {
                    0
                } else {
                    self.calc_compact_u16_encoded_size(&new_accounts, Some(32)) - 1
                };

                // We have to use MAX_TX_PACK_SIZE because it appears we cannnot send tx with a
                // size of exactly 1232 bytes.
                // Also, some logs may get truncated near the end of the tx, so we need to leave
                // some room for that.
                let cu_to_user_per_fill = if self.use_burst_cu_limit {
                    BURST_CU_PER_FILL
                } else {
                    CU_PER_FILL
                };

                // ensure at least 1 attempted fill
                if (running_tx_size + new_ix_cost + additional_accounts_cost >= MAX_TX_PACK_SIZE
                    || running_cu_used + cu_to_user_per_fill >= MAX_CU_PER_TX)
                    && ixs.len() > starting_ixs_size + 1
                {
                    log::info!("Fully packed fill tx (ixs: {}): est. tx size {}, max: {MAX_TX_PACK_SIZE}, est. CU used: expected {}, max {MAX_CU_PER_TX}, (fill_tx_id: {fill_tx_id}", ixs.len(), running_tx_size + new_ix_cost + additional_accounts_cost, running_cu_used + cu_to_user_per_fill);
                    break;
                }

                // add to tx
                // log::info!("");
                let instructions: Vec<Instruction> =
                    instructions.into_iter().map(|ix| ix.clone()).collect();
                ixs.extend(instructions);
                running_tx_size += new_ix_cost + additional_accounts_cost;
                running_cu_used += cu_to_user_per_fill;

                for new_account in new_accounts {
                    unique_accounts.insert(new_account.pubkey);
                }
                idx_used += 1;
                nodes_sent.push(node_to_fill);
            }
        }

        if idx_used == 0 {
            return Ok(nodes_sent.len());
        }

        if nodes_sent.is_empty() {
            return Ok(0);
        }

        if let Some(true) = self.revert_on_failure {
            builder.revert_fill(*user_account_pubkey);
        }

        // let recent_blockhash = self
        //     .drift_client
        //     .backend
        //     .rpc_client
        //     .get_latest_blockhash()
        //     .await
        //     .expect("get recent blockhash");
        let recent_blockhash = self.blockhash_subscriber.get_latest_blockhash().await;

        let mut params = SimulateAndGetTxWithCUsParams {
            connection: self.drift_client.backend.rpc_client.clone(),
            payer: self.drift_client.wallet.signer.clone(),
            lookup_table_accounts: vec![self
                .lookup_table_account
                .clone()
                .expect("lookup table account")
                .clone()],
            ixs: ixs.into(),
            cu_limit_multiplier: Some(SIM_CU_ESTIMATE_MULTIPLIER),
            do_simulation: Some(true),
            recent_blockhash: Some(recent_blockhash),
            dump_tx: None,
        };

        let sim_res = simulate_and_get_tx_with_cus(&mut params)
            .await
            .expect("simulate");

        // ERROR:
        if self.simulate_tx_for_cu_estimate.is_some() && sim_res.sim_error.is_some() {
            log::error!(
                "sim_error: {} (fill_tx_id: {fill_tx_id})",
                sim_res.sim_error.unwrap()
            );
        } else {
            if self.dry_run {
                log::info!("dry run, not sending tx (fill_tx_id: {fill_tx_id}");
            } else {
                let nodes_sent: Vec<NodeToFill> = nodes_sent
                    .clone()
                    .into_iter()
                    .map(|node| node.clone())
                    .collect();
                if self.has_enough_sol_to_fill {
                    self.send_fill_tx_and_parse_logs(
                        fill_tx_id,
                        &nodes_sent,
                        sim_res.tx,
                        build_for_bundle,
                    )
                    .await;
                } else {
                    log::info!("not sending tx because we don't have enough SOL to fill (fill_tx_id: {fill_tx_id}");
                }
            }
        }

        Ok(nodes_sent.len())
    }

    async fn filter_perp_nodes_for_market(
        &mut self,
        fillable_nodes: &[NodeToFill],
        triggerable_nodes: &[Node],
    ) -> (Vec<NodeToFill>, Vec<Node>) {
        let mut seen_fillable_nodes = HashSet::new();
        let mut filtered_fillable_nodes = Vec::new();
        for node in fillable_nodes {
            let sig = get_node_to_fill_signature(node);
            if seen_fillable_nodes.contains(&sig) {
                continue;
            }
            seen_fillable_nodes.insert(sig);
            if self.filter_fillable_nodes(node).await {
                filtered_fillable_nodes.push(node.clone());
            }
        }

        let mut seen_triggerable_nodes = HashSet::new();
        let mut filtered_triggerable_nodes = Vec::new();
        for node in triggerable_nodes {
            let sig = get_node_to_trigger_signature(node);
            if seen_triggerable_nodes.contains(&sig) {
                continue;
            }
            seen_triggerable_nodes.insert(sig);
            if self.filter_triggerable_nodes(node) {
                filtered_triggerable_nodes.push(*node);
            }
        }

        (filtered_fillable_nodes, filtered_triggerable_nodes)
    }

    async fn execute_fillable_perp_nodes_for_market(
        &mut self,
        fillable_nodes: &[NodeToFill],
        build_for_bundle: bool,
    ) {
        self.try_fill_perp_nodes(fillable_nodes, build_for_bundle)
            .await;
    }

    async fn execute_triggerable_perp_nodes_for_market(
        &mut self,
        triggerable_nodes: &[Node],
        build_for_bundle: bool,
    ) {
        let drift_client = self.drift_client.clone();
        let authority = drift_client.wallet().authority();
        for node_to_trigger in triggerable_nodes {
            let user_account = node_to_trigger.get_user_account();
            let user = self.get_user_account_and_slot_from_map(user_account).await;
            let order = node_to_trigger.get_order();
            if let Some((user, slot)) = user {
                log::info!(
                    "trying to trigger (account: {}, slot: {}) order {}",
                    user_account,
                    slot,
                    order.order_id
                );

                let node_sig = get_node_to_trigger_signature(node_to_trigger);
                self.triggering_nodes.insert(node_sig, Instant::now());

                let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)];
                ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
                    self.priority_fee_subscriber.get_custom_strategy_result() as u64,
                ));

                let mut builder = drift_client
                    .init_tx(authority, false)
                    .expect("build tx")
                    .trigger_order_ix(&user_account, &user, order, None, vec![]);

                if let Some(true) = self.revert_on_failure {
                    builder = builder.revert_fill(*authority);
                }

                ixs.extend(builder.instructions().to_vec());

                let recent_blockhash = drift_client
                    .backend
                    .rpc_client
                    .get_latest_blockhash()
                    .await
                    .expect("get recent blockhash");

                let mut params = SimulateAndGetTxWithCUsParams {
                    connection: drift_client.backend.rpc_client.clone(),
                    payer: drift_client.wallet.signer.clone(),
                    lookup_table_accounts: vec![self
                        .lookup_table_account
                        .clone()
                        .expect("lookup table account")
                        .clone()],
                    ixs: ixs.into(),
                    cu_limit_multiplier: Some(SIM_CU_ESTIMATE_MULTIPLIER),
                    do_simulation: Some(true),
                    recent_blockhash: Some(recent_blockhash),
                    dump_tx: None,
                };

                let sim_res = simulate_and_get_tx_with_cus(&mut params)
                    .await
                    .expect("simulate");

                if self.simulate_tx_for_cu_estimate.is_some() && sim_res.sim_error.is_some() {
                    log::error!(
                        "execute_triggerable_perp_nodes_for_market sim_error: {})",
                        sim_res.sim_error.unwrap()
                    );
                } else {
                    if self.dry_run {
                        log::info!("dry run, not triggering node");
                    } else {
                        if self.has_enough_sol_to_fill {
                            let tx_sig = sim_res.tx.signatures[0];
                            self.register_tx_sig_to_confirm(
                                tx_sig,
                                Instant::now(),
                                &[],
                                u16::MAX,
                                TxType::Trigger,
                            );

                            if build_for_bundle {
                                self.send_tx_through_jito(
                                    &sim_res.tx,
                                    "trigger_order",
                                    Some(sim_res.tx.signatures[0]),
                                )
                                .await;
                            } else {
                                match drift_client.sign_and_send(sim_res.tx.message, false).await {
                                    Ok(sig) => {
                                        log::info!("Signature: {sig}");
                                    }
                                    Err(e) => {
                                        log::error!("Error ({e}) triggering order for user (account: {user_account}) order: {}", order.order_id);
                                    }
                                }
                            }
                        } else {
                            log::info!("Not enough SOL to trigger, not triggering node");
                        }
                    }
                }
            }
        }
    }

    async fn settle_pnls(&mut self) {
        // Check if we have enough SOL to fill
        let authority = self.drift_client.wallet().authority();
        let filler_sol_balance = self
            .drift_client
            .backend
            .rpc_client
            .get_balance(authority)
            .await
            .expect("get balance");
        log::warn!(
            "Minimum gas balance to fill: {}",
            self.min_gas_balance_to_fill
        );
        self.has_enough_sol_to_fill = filler_sol_balance as f64 >= self.min_gas_balance_to_fill;
    }

    fn using_jito(&self) -> bool {
        self.bundle_sender.is_some()
    }

    fn can_send_outside_jito(&self) -> bool {
        if let Some(sender) = &self.bundle_sender {
            return self.using_jito()
                || matches!(
                    sender.strategy,
                    JitoStrategy::NonJitoOnly | JitoStrategy::Hybrid
                );
        }

        false
    }

    fn slots_until_jito_leader(&self) -> Option<u64> {
        if !self.using_jito() {
            return None;
        }

        match &self.bundle_sender {
            Some(sender) => sender.slots_until_next_leader(),
            None => None,
        }
    }

    fn should_build_for_bundle(&self) -> bool {
        if !self.using_jito() {
            return false;
        }

        if let Some(true) = self.global_config.only_send_during_jito_leader {
            match self.slots_until_jito_leader() {
                Some(slots) => return slots < SLOTS_UNTIL_JITO_LEADER_TO_SEND,
                None => return false,
            }
        }

        true
    }

    async fn try_fill(&mut self) {
        let _start_time = Instant::now();
        // let mut ran = false;

        if !self.has_enough_sol_to_fill {
            log::info!("Not enough SOL to fill, skipping fill");
            return;
        }

        let _user = self.drift_client.get_user(None);

        let mut dlob = self.get_dlob().await;
        self.prune_throttled_node();

        // 1) get all fillable nodes
        let mut fillable_nodes = Vec::new();
        let mut triggerable_nodes = Vec::new();
        for market in self.drift_client.get_perp_market_accounts() {
            if let Some(ref mut dlob) = dlob {
                match self.get_perp_nodes_for_market(market, dlob).await {
                    Ok((nodes_to_fill, nodes_to_trigger)) => {
                        fillable_nodes.extend(nodes_to_fill);
                        triggerable_nodes.extend(nodes_to_trigger);
                    }
                    Err(e) => {
                        log::warn!(
                            "{}: :x: Failed to get fillable nodes for market {}, Error: {e}",
                            self.name,
                            market.market_index
                        );
                        continue;
                    }
                }
            }
        }

        // filler out nodes that we know can not be filled
        let (filtered_fillable_nodes, filtered_triggerable_nodes) = self
            .filter_perp_nodes_for_market(&fillable_nodes, &triggerable_nodes)
            .await;
        log::debug!(
            "filtered fillable nodes from {} to {}, filtered triggerable nodes from {} to {}",
            fillable_nodes.len(),
            filtered_fillable_nodes.len(),
            triggerable_nodes.len(),
            filtered_triggerable_nodes.len()
        );

        let build_bundle = self.should_build_for_bundle();

        self.execute_fillable_perp_nodes_for_market(&filtered_fillable_nodes, build_bundle)
            .await;
        self.execute_triggerable_perp_nodes_for_market(&filtered_triggerable_nodes, build_bundle)
            .await;

        // ran = true;
    }
}
