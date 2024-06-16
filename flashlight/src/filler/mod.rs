use std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use log::info;
use sdk::{
    accounts::BulkAccountLoader,
    blockhash_subscriber::BlockhashSubscriber,
    dlob::dlob_subscriber::DLOBSubscriber,
    drift_client::DriftClient,
    jupiter::JupiterClient,
    priority_fee::priority_fee_subscriber::PriorityFeeSubscriber,
    slot_subscriber::SlotSubscriber,
    user_config::UserSubscriptionConfig,
    usermap::{user_stats_map::UserStatsMap, UserMap},
    AccountProvider,
};
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
};

use crate::{
    bundle_sender::BundleSender,
    config::{FillerConfig, GlobalConfig},
    metrics::RuntimeSpec,
    util::{valid_minimum_gas_amount, valid_rebalance_settled_pnl_threshold},
};

const DEFAULT_INTERVAL_MS: u16 = 6000;

struct FillerBot<'a, T, U>
where
    T: AccountProvider,
{
    name: String,
    dry_run: bool,
    // default_interval_ms: u16,
    slot_subscriber: SlotSubscriber,
    bulk_account_loader: Option<BulkAccountLoader>,
    // user_stats_map_subscription_config: &'a UserSubscriptionConfig<U>,
    drift_client: Arc<DriftClient<T, U>>,
    /// Connection to use specifically for confirming transactions
    // tx_confirmation_connection: RpcClient,
    polling_interval_ms: u16,
    revert_on_failure: Option<bool>,
    simulate_tx_for_cu_estimate: Option<bool>,
    lookup_table_account: Option<AddressLookupTableAccount>,
    bundle_sender: Option<BundleSender>,

    filler_config: FillerConfig,
    global_config: GlobalConfig,
    dlob_subscriber: Option<DLOBSubscriber<T, U>>,

    user_map: Option<UserMap>,
    user_stats_map: Option<UserStatsMap<T, U>>,

    // periodic_task_mutex = new Mutex();

    // watchdogTimerMutex = new Mutex();
    watchdog_timer_last_pat_time: Instant,

    interval_ids: Vec<Instant>,
    throttled_nodes: HashMap<String, u16>,
    filling_nodes: HashMap<String, u16>,
    triggering_nodes: HashMap<String, u16>,

    use_burst_cu_limit: bool,
    fill_tx_since_burst_cu: u16,
    fill_tx_id: u16,
    last_settle_pnl: Instant,

    priority_fee_subscriber: PriorityFeeSubscriber<T, U>,
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
    // expiredNodesSet: LRUCache<string, boolean>;
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

impl<'a, T, U> FillerBot<'a, T, U>
where
    T: AccountProvider,
{
    pub fn new(
        slot_subscriber: SlotSubscriber,
        bulk_account_loader: Option<BulkAccountLoader>,
        drift_client: Arc<DriftClient<T, U>>,
        user_map: UserMap,
        runtime_spec: RuntimeSpec,
        global_config: GlobalConfig,
        filler_config: FillerConfig,
        mut priority_fee_subscriber: PriorityFeeSubscriber<T, U>,
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

        Self {
            global_config,
            filler_config: filler_config.clone(),
            name: filler_config.base_config.bot_id,
            dry_run: filler_config.base_config.dry_run,
            slot_subscriber,
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

    pub fn base_init(&mut self) {
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

        // self.clock_subscriber.subscribe().await;
    }
}
