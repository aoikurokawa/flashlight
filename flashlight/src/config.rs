use sdk::types::Context as DriftEnv;
use solana_sdk::pubkey::Pubkey;

use crate::types::JitoStrategy;

#[allow(dead_code)]
#[derive(Debug)]
pub enum TxSenderType {
    Fast,
    Retry,
    WhileValid,
}

#[derive(Debug, Clone, Default)]
pub struct BaseBotConfig {
    pub bot_id: String,

    pub dry_run: bool,

    pub metrics_port: Option<u16>,

    pub run_once: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct FillerConfig {
    pub base_config: BaseBotConfig,

    pub filler_polling_interval: Option<u16>,

    pub revert_on_failure: Option<bool>,

    pub simulate_tx_for_cu_estimate: Option<bool>,

    pub rebalance_filler: Option<bool>,

    pub rebalance_settled_pnl_threshold: Option<f64>,

    pub min_gas_balance_to_fill: Option<f64>,
}

#[derive(Debug, Default)]
pub struct GlobalConfig {
    pub drift_env: Option<DriftEnv>,

    pub endpoint: Option<String>,

    pub ws_endpoint: Option<String>,

    /// endpoint to use helius priority fee strategy
    pub helius_endpoint: Option<String>,

    ///onal rpc endpoints to send transactions to
    pub additional_send_tx_endpoints: Option<Vec<String>>,

    ///nt to confirm txs on
    pub tx_confirmation_endpoint: Option<String>,

    ///t metrics port to use, will be overridden by {@link BaseBotConfig.metricsPort} if provided
    pub metrics_port: Option<u16>,

    ///e all metrics
    pub disable_metrics: Option<bool>,

    pub priority_fee_method: Option<String>,

    pub max_priority_fee_micro_lamports: Option<u16>,

    pub resub_timeout_ms: Option<u16>,

    pub priority_fee_multiplier: Option<u16>,

    pub keeper_private_key: Option<Pubkey>,

    pub init_user: Option<bool>,

    pub test_liveness: Option<bool>,

    pub cancel_open_orders: Option<bool>,

    pub close_open_positions: Option<bool>,

    pub force_deposit: Option<u16>,

    pub websocket: Option<bool>,

    pub event_subscriber: Option<bool>,

    pub run_once: Option<bool>,

    pub debug: Option<bool>,

    pub subaccounts: Option<Vec<u16>>,

    pub event_subscriber_polling_interval: u16,

    pub bulk_account_loader_polling_interval: u16,

    pub use_jito: Option<bool>,

    pub jito_strategy: Option<JitoStrategy>,

    pub jito_block_engine_url: Option<String>,

    pub jito_auth_private_key: Option<Pubkey>,

    pub jito_min_bundle_tip: Option<u16>,

    pub jito_max_bundle_tip: Option<u16>,

    pub jito_max_bundle_fail_count: Option<u16>,

    pub jito_tip_multiplier: Option<u16>,

    pub only_send_during_jito_leader: Option<bool>,

    pub tx_retry_timeout_ms: Option<u16>,

    pub tx_sender_type: Option<TxSenderType>,

    pub tx_skip_preflight: Option<bool>,

    pub tx_max_retries: Option<u16>,

    pub rebalance_filler: Option<bool>,
}
