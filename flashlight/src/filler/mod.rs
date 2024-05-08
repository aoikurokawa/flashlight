use drift_sdk::slot_subscriber::SlotSubscriber;

use crate::config::{FillerConfig, GlobalConfig};

struct FillerBot {
    name: String,
    dry_run: bool,
    default_interval_ms: u16,

	slot_subscriber: SlotSubscriber,
	bulkAccountLoader: Option<BulkAccountLoader>,
	protected userStatsMapSubscriptionConfig: UserSubscriptionConfig;
	protected driftClient: DriftClient;
	/// Connection to use specifically for confirming transactions
	protected txConfirmationConnection: Connection;
	protected pollingIntervalMs: number;
	protected revertOnFailure?: boolean;
	protected simulateTxForCUEstimate?: boolean;
	protected lookupTableAccount?: AddressLookupTableAccount;
	protected bundleSender?: BundleSender;

	private fillerConfig: FillerConfig;
	private globalConfig: GlobalConfig;
	private dlobSubscriber?: DLOBSubscriber;

	private userMap?: UserMap;
	protected userStatsMap?: UserStatsMap;

	protected periodicTaskMutex = new Mutex();

	protected watchdogTimerMutex = new Mutex();
	protected watchdogTimerLastPatTime = Date.now();

	protected intervalIds: Array<NodeJS.Timer> = [];
	protected throttledNodes = new Map<string, number>();
	protected fillingNodes = new Map<string, number>();
	protected triggeringNodes = new Map<string, number>();

	protected useBurstCULimit = false;
	protected fillTxSinceBurstCU = 0;
	protected fillTxId = 0;
	protected lastSettlePnl = Date.now() - SETTLE_POSITIVE_PNL_COOLDOWN_MS;

	protected priorityFeeSubscriber: PriorityFeeSubscriber;
	protected blockhashSubscriber: BlockhashSubscriber;
	/// stores txSigs that need to been confirmed in a slower loop, and the time they were confirmed
	protected pendingTxSigsToconfirm: LRUCache<
		string,
		{
			ts: number;
			nodeFilled: Array<NodeToFill>;
			fillTxId: number;
			txType: TxType;
		}
	>;
	protected expiredNodesSet: LRUCache<string, boolean>;
	protected confirmLoopRunning = false;
	protected confirmLoopRateLimitTs =
		Date.now() - CONFIRM_TX_RATE_LIMIT_BACKOFF_MS;

	protected jupiterClient?: JupiterClient;

	// metrics
	protected metricsInitialized = false;
	protected metricsPort?: number;
	protected metrics?: Metrics;
	protected bootTimeMs?: number;

	protected runtimeSpec: RuntimeSpec;
	protected runtimeSpecsGauge?: GaugeValue;
	protected tryFillDurationHistogram?: HistogramValue;
	protected estTxCuHistogram?: HistogramValue;
	protected simulateTxHistogram?: HistogramValue;
	protected lastTryFillTimeGauge?: GaugeValue;
	protected mutexBusyCounter?: CounterValue;
	protected sentTxsCounter?: CounterValue;
	protected attemptedTriggersCounter?: CounterValue;
	protected landedTxsCounter?: CounterValue;
	protected txSimErrorCounter?: CounterValue;
	protected pendingTxSigsToConfirmGauge?: GaugeValue;
	protected pendingTxSigsLoopRateLimitedCounter?: CounterValue;
	protected evictedPendingTxSigsToConfirmCounter?: CounterValue;
	protected expiredNodesSetSize?: GaugeValue;
	protected jitoBundlesAcceptedGauge?: GaugeValue;
	protected jitoBundlesSimulationFailureGauge?: GaugeValue;
	protected jitoDroppedBundleGauge?: GaugeValue;
	protected jitoLandedTipsGauge?: GaugeValue;
	protected jitoBundleCount?: GaugeValue;

	protected hasEnoughSolToFill: boolean = false;
	protected rebalanceFiller: boolean;
	protected minGasBalanceToFill: number;
	protected rebalanceSettledPnlThreshold: BN;
}

impl FillerBot {
    pub fn new(global_config: GlobalConfig, filler_config: FillerConfig) -> Self {}
}
