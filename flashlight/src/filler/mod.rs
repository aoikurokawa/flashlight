use std::{collections::HashMap, time::Instant};

use drift::state::user_map::UserStatsMap;
use sdk::{
    accounts::BulkAccountLoader, blockhash_subscriber::BlockhashSubscriber,
    dlob::dlob_subscriber::DLOBSubscriber, jupiter::JupiterClient,
    priority_fee::priority_fee_subscriber::PriorityFeeSubscriber, slot_subscriber::SlotSubscriber,
    user_config::UserSubscriptionConfig, usermap::UserMap, AccountProvider, DriftClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;

use crate::{
    bundle_sender::BundleSender,
    config::{FillerConfig, GlobalConfig},
};

struct FillerBot<'a, T, D, S, F, Fut, G>
where
    T: AccountProvider,
    S: sdk::dlob::types::SlotSource,
    D: sdk::dlob::types::DLOBSource,
{
    name: String,
    dry_run: bool,
    default_interval_ms: u16,

    slot_subscriber: SlotSubscriber,
    bulk_account_loader: Option<BulkAccountLoader<F, Fut, G>>,
    user_stats_map_subscription_config: UserSubscriptionConfig<T>,
    drift_client: DriftClient<T>,
    /// Connection to use specifically for confirming transactions
    tx_confirmation_connection: RpcClient,
    polling_interval_ms: u16,
    revert_on_failure: Option<bool>,
    simulate_tx_for_cu_estimate: Option<bool>,
    lookup_table_account: Option<AddressLookupTableAccount>,
    bundle_sender: Option<BundleSender>,

    filler_config: FillerConfig,
    global_config: GlobalConfig,
    dlob_subscriber: Option<DLOBSubscriber<T, D, S>>,

    user_map: Option<UserMap>,
    user_stats_map: Option<UserStatsMap<'a>>,

    // periodic_task_mutex = new Mutex();

    // watchdogTimerMutex = new Mutex();
    watchdog_timer_last_pat_time: std::time::SystemTime,

    interval_ids: Vec<Instant>,
    throttled_nodes: HashMap<String, u16>,
    filling_nodes: HashMap<String, u16>,
    triggering_nodes: HashMap<String, u16>,

    use_burst_cu_limit: bool,
    fill_tx_since_burst_cu: u16,
    fill_tx_id: u16,
    last_settle_pnl: std::time::SystemTime,

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
    // expiredNodesSet: LRUCache<string, boolean>;
    confirm_loop_running: bool,
    confirm_loop_rate_limit_ts: std::time::SystemTime,

    jupiter_client: Option<JupiterClient>,

    // metrics
    // metrics_initialized: bool,
    // metrics_port: Option<u16>,
    // metrics: Option<Metrics>,
    // boot_time_ms: Option<u16>,

    // runtime_spec: RuntimeSpec,
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
    min_gas_balance_to_fill: u16,
    // rebalance_settled_pnl_threshold: BN;
}

impl<T, D, S, F, Fut, G> FillerBot<'_, T, D, S, F, Fut, G>
where
    T: AccountProvider,
    S: sdk::dlob::types::SlotSource,
    D: sdk::dlob::types::DLOBSource,
{
    pub fn new(
        slot_subscriber: SlotSubscriber,
        bulk_account_loader: Option<BulkAccountLoader<F, Fut, G>>,
        drift_client: DriftClient<T>,
        user_map: UserMap,
        global_config: GlobalConfig,
        filler_config: FillerConfig,
        priority_fee_subscriber: PriorityFeeSubscriber<T>,
        blockhash_subscriber: BlockhashSubscriber,
        bundle_sender: Option<BundleSender>,
    ) -> Self {
        // todo!()
        let tx_confirmation_connection = match global_config.tx_confirmation_endpoint {
            Some(endpoint) => RpcClient::new(endpoint),
            None => drift_client.backend.rpc_client,
        };

        let user_stats_map_subscription_config = match bulk_account_loader {
            Some(account_loader) => {
                let loader =
                    BulkAccountLoader::new(drift_client.backend.rpc_client, polling_frequency);
                UserSubscriptionConfig::Polling {
                    account_loader: loader,
                }
            }
            None => {
                drift_client.get_user_stats(authority)
            }
        };

        Self {
            global_config,
            filler_config,
            name: filler_config.base_config.bot_id,
            dry_run: filler_config.base_config.dry_run,
            slot_subscriber,
            tx_confirmation_connection,
            bulk_account_loader,
            user_stats_map_subscription_config,
        }
    }
}
