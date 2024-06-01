use sdk::types::Context as DriftEnv;
use solana_sdk::pubkey::Pubkey;

use crate::types::JitoStrategy;

enum TxSenderType {
    Fast,
    Retry,
    WhileValid,
}

#[derive(Debug, Clone)]
pub struct BaseBotConfig {
    pub bot_id: String,

    pub dry_run: bool,

    pub metrics_port: Option<u16>,

    pub run_once: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct FillerConfig {
    pub(crate) base_config: BaseBotConfig,

    pub(crate) filler_polling_interval: Option<u16>,

    pub(crate) revert_on_failure: Option<bool>,

    pub(crate) simulate_tx_for_cu_estimate: Option<bool>,

    pub(crate) rebalance_filler: Option<bool>,

    pub(crate) rebalance_settled_pnl_threshold: Option<f64>,

    pub(crate) min_gas_balance_to_fill: Option<f64>,
}

pub(crate) struct GlobalConfig {
    pub(crate) drift_env: Option<DriftEnv>,

    pub(crate) endpoint: Option<String>,

    pub(crate) ws_endpoint: Option<String>,

    /// helius endpoint to use helius priority fee strategy
    pub(crate) helius_endpoint: Option<String>,

    /// additional rpc endpoints to send transactions to
    pub(crate) additional_send_tx_endpoints: Option<Vec<String>>,

    /// endpoint to confirm txs on
    pub(crate) tx_confirmation_endpoint: Option<String>,

    /// default metrics port to use, will be overridden by {@link BaseBotConfig.metricsPort} if provided
    pub(crate) metrics_port: Option<u16>,

    /// disable all metrics
    pub(crate) disable_metrics: Option<bool>,

    pub(crate) priority_fee_method: Option<String>,

    pub(crate) max_priority_fee_micro_lamports: Option<u16>,

    pub(crate) resub_timeout_ms: Option<u16>,

    pub(crate) priority_fee_multiplier: Option<u16>,

    pub(crate) keeper_private_key: Option<Pubkey>,

    pub(crate) init_user: Option<bool>,

    pub(crate) test_liveness: Option<bool>,

    pub(crate) cancel_open_orders: Option<bool>,

    pub(crate) close_open_positions: Option<bool>,

    pub(crate) force_deposit: Option<u16>,

    pub(crate) websocket: Option<bool>,

    pub(crate) event_subscriber: Option<bool>,

    pub(crate) run_once: Option<bool>,

    pub(crate) debug: Option<bool>,

    pub(crate) subaccounts: Option<Vec<u16>>,

    pub(crate) event_subscriber_polling_interval: u16,

    pub(crate) bulk_account_loader_polling_interval: u16,

    pub(crate) use_jito: Option<bool>,

    pub(crate) jito_strategy: Option<JitoStrategy>,

    pub(crate) jito_block_engine_url: Option<String>,

    pub(crate) jito_auth_private_key: Option<Pubkey>,

    pub(crate) jito_min_bundle_tip: Option<u16>,

    pub(crate) jito_max_bundle_tip: Option<u16>,

    pub(crate) jito_max_bundle_fail_count: Option<u16>,

    pub(crate) jito_tip_multiplier: Option<u16>,

    pub(crate) only_send_during_jito_leader: Option<bool>,

    pub(crate) tx_retry_timeout_ms: Option<u16>,

    pub(crate) tx_sender_type: Option<TxSenderType>,

    pub(crate) tx_skip_preflight: Option<bool>,

    pub(crate) tx_max_retries: Option<u16>,

    pub(crate) rebalance_filler: Option<bool>,
}
