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
    user::{MarketType, OrderType},
};
use log::info;
use lru::LruCache;
use sdk::{
    accounts::BulkAccountLoader,
    blockhash_subscriber::BlockhashSubscriber,
    clock::clock_subscriber::ClockSubscriber,
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
    usermap::{user_stats_map::UserStatsMap, UserMap},
    AccountProvider,
};
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    clock::Clock,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
};

use crate::{
    bundle_sender::BundleSender,
    config::{FillerConfig, GlobalConfig},
    metrics::RuntimeSpec,
    util::{
        get_fill_signature_from_user_account_and_orader_id, get_node_to_fill_signature,
        get_node_to_trigger_signature, valid_minimum_gas_amount,
        valid_rebalance_settled_pnl_threshold,
    },
};

const DEFAULT_INTERVAL_MS: u16 = 6000;
const FILL_ORDER_THROTTLE_BACKOFF: u64 = 1000; // the time to wait before trying to fill a throttled (error filling) node again
const THROTTLED_NODE_SIZE_TO_PRUNE: usize = 10; // Size of throttled nodes to get to before pruning the map
const TRIGGER_ORDER_COOLDOWN_MS: u64 = 1000; // the time to wait before trying to a node in the triggering map again

const EXPIRE_ORDER_BUFFER_SEC: i64 = 60; // add extra time before trying to expire orders (want to avoid 6252 error due to clock drift)

struct FillerBot<'a, T>
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
    // tx_confirmation_connection: RpcClient,
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
    // protected pendingTxSigsToconfirm: LRUCache<
    // 	string,
    // 	{
    // 		ts: number;
    // 		nodeFilled: Array<NodeToFill>;
    // 		fillTxId: number;
    // 		txType: TxType;
    // 	}
    // >;
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

impl<'a, T> FillerBot<'a, T>
where
    T: AccountProvider + Clone,
{
    pub async fn new(
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
        // todo!()
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

        let pubsub_client = PubsubClient::new("wss://api.devnet.solana.com/")
            .await
            .expect("init pubsub client");

        Self {
            global_config,
            filler_config: filler_config.clone(),
            name: filler_config.base_config.bot_id,
            dry_run: filler_config.base_config.dry_run,
            slot_subscriber,
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
            expired_nodes_set: LruCache::new(NonZeroUsize::new(100).unwrap()),
            confirm_loop_running: false,
            confirm_loop_rate_limit_ts: Instant::now() - Duration::from_secs(5_000),
            dlob_subscriber: None,
            drift_client,
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

    pub async fn base_init(&mut self) {
        let start_init_user_stats_map = Instant::now();
        info!("Initializing user_stats_map");

        let user_stats_loader = BulkAccountLoader::new(
            self.drift_client.backend.rpc_client.clone(),
            CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            },
            Duration::from_secs(0),
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

        self.dlob_subscriber = Some(DLOBSubscriber::new(DLOBSubscriptionConfig {
            drift_client,
            dlob_source: DlobSource::UserMap(user_map),
            slot_source: SlotSource::SlotSubscriber(slot_subscriber),
            update_frequency: Duration::from_millis((self.polling_interval_ms - 500) as u64),
        }));

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
        // self.try
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
    ) -> Option<(Vec<NodeToFill>, Vec<Node>)> {
        let market_index = market.market_index;

        let oracle = self
            .drift_client
            .get_oracle_price_data_and_slot_for_perp_market(market_index);
        if let Some(oracle) = oracle {
            let v_ask = calculate_ask_price(&market, &oracle.data).expect("calculate ask price");
            let v_bid = calculate_bid_price(&market, &oracle.data).expect("calculate bid price");

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
                    &MarketAccount::PerpMarket(perp_market),
                )
                .expect("find nodes to fill");

            let nodes_to_trigger = dlob.find_nodes_to_trigger(
                market_index,
                oracle.data.price as u64,
                MarketType::Perp,
                self.drift_client.get_state_account(),
            );

            return Some((nodes_to_fill, nodes_to_trigger));
        }

        None
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

    fn is_dlob_node_throttled(&self, dlob_node: Node) -> bool {
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

    fn prune_throttled_node(&mut self) {
        if self.throttled_nodes.len() > THROTTLED_NODE_SIZE_TO_PRUNE {
            let now = Instant::now();
            let duration_threshold = Duration::new(2_u64 * FILL_ORDER_THROTTLE_BACKOFF, 0);

            self.throttled_nodes
                .retain(|_, v| *v + duration_threshold <= now)
        }
    }

    async fn filter_fillable_nodes(&self, node_to_fill: &NodeToFill) -> bool {
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
        if self.is_dlob_node_throttled(node_to_fill.get_node()) {
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

    async fn filter_perp_nodes_for_market(
        &self,
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

    async fn try_fill(&mut self) {
        let start_time = Instant::now();
        let ran = false;

        if !self.has_enough_sol_to_fill {
            log::info!("Not enough SOL to fill, skipping fill");
            return;
        }

        let user = self.drift_client.get_user(None);

        let mut dlob = self.get_dlob().await;
        self.prune_throttled_node();

        // 1) get all fillable nodes
        let mut fillable_nodes = Vec::new();
        let mut triggerable_nodes = Vec::new();
        for market in self.drift_client.get_perp_market_accounts() {
            if let Some(ref mut dlob) = dlob {
                match self.get_perp_nodes_for_market(market, dlob).await {
                    Some((nodes_to_fill, nodes_to_trigger)) => {
                        fillable_nodes.extend(nodes_to_fill);
                        triggerable_nodes.extend(nodes_to_trigger);
                    }
                    None => {
                        log::warn!(
                            "{}: :x: Failed to get fillable nodes for market {}",
                            self.name,
                            market.market_index
                        );
                        continue;
                    }
                }
            }
        }

        // filler out nodes that we know can not be filled
        self.filter_perp_nodes_for_market(&fillable_nodes, &triggerable_nodes);
    }
}
