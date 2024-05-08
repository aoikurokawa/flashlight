use drift_sdk::{types::Context as DriftEnv, Pubkey};

enum JitoStrategy {
    JitoOnly,
    NonJitoOnly,
    Hybrid,
}

enum TxSenderType {
    Fast,
    Retry,
    WhileValid,
}

pub(crate) struct BaseBotConfig {
    bot_id: String,

    dry_run: bool,

    metrics_port: Option<u16>,

    run_once: Option<bool>,
}

pub(crate) struct FillerConfig {
    base_config: BaseBotConfig,

    filler_polling_interval: Option<u16>,

    revert_on_failure: Option<bool>,

    simulate_tx_for_cu_estimate: Option<bool>,

    rebalance_filler: Option<bool>,

    rebalance_settled_pnl_threshold: Option<u16>,

    min_gas_balance_to_fill: Option<u16>,
}

pub(crate) struct GlobalConfig {
    drift_env: Option<DriftEnv>,

    endpoint: Option<String>,

    ws_endpoint: Option<String>,

    /// helius endpoint to use helius priority fee strategy
    helius_endpoint: Option<String>,

    /// additional rpc endpoints to send transactions to
    additional_send_tx_endpoints: Option<Vec<String>>,

    /// endpoint to confirm txs on
    tx_confirmation_endpoint: Option<String>,

    /// default metrics port to use, will be overridden by {@link BaseBotConfig.metricsPort} if provided
    metrics_port: Option<u16>,

    /// disable all metrics
    disable_metrics: Option<bool>,

    priority_fee_method: Option<String>,

    max_priority_fee_micro_lamports: Option<u16>,

    resub_timeout_ms: Option<u16>,

    priority_fee_multiplier: Option<u16>,

    keeper_private_key: Option<Pubkey>,

    init_user: Option<bool>,

    test_liveness: Option<bool>,

    cancel_open_orders: Option<bool>,

    close_open_positions: Option<bool>,

    force_deposit: Option<u16>,

    websocket: Option<bool>,

    event_subscriber: Option<bool>,

    run_once: Option<bool>,

    debug: Option<bool>,

    subaccounts: Option<Vec<u16>>,

    event_subscriber_polling_interval: u16,

    bulk_account_loader_polling_interval: u16,

    use_jito: Option<bool>,

    jito_strategy: Option<JitoStrategy>,

    jito_block_engine_url: Option<String>,

    jito_auth_private_key: Option<Pubkey>,

    jito_min_bundle_tip: Option<u16>,

    jito_max_bundle_tip: Option<u16>,

    jito_max_bundle_fail_count: Option<u16>,

    jito_tip_multiplier: Option<u16>,

    only_send_during_jito_leader: Option<bool>,

    tx_retry_timeout_ms: Option<u16>,

    tx_sender_type: Option<TxSenderType>,

    tx_skip_preflight: Option<bool>,

    tx_max_retries: Option<u16>,

    rebalance_filler: Option<bool>,
}
