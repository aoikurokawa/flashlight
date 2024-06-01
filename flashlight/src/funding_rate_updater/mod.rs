use std::time::Instant;

use sdk::{
    config::DriftEnv,
    priority_fee::{
        drift_priority_fee_method::DriftMarketInfo,
        priority_fee_subscriber_map::PriorityFeeSubscriberMap,
        types::PriorityFeeSubscriberMapConfig,
    },
    AccountProvider, DriftClient,
};
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;

use crate::{config::BaseBotConfig, util::get_drift_priority_fee_endpoint};

pub struct FundingRateUpdaterBot<'a, T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    run_once: bool,
    default_interval_ms: u64,

    drift_client: &'a DriftClient<T, U>,
    interval_ids: Vec<u64>,
    priority_fee_subscriber_map: PriorityFeeSubscriberMap,
    lookup_table_account: Option<AddressLookupTableAccount>,

    watchdog_timer_last_par_time: Instant,
    in_progress: bool,
}

impl<'a, T: AccountProvider, U> FundingRateUpdaterBot<'a, T, U> {
    pub fn new(drift_client: &'a DriftClient<T, U>, config: BaseBotConfig) -> Self {
        let drift_markets = DriftMarketInfo {
            market_type: "perp".to_string(),
            market_index: 1,
        };
        let priority_config = PriorityFeeSubscriberMapConfig {
            frequency_ms: Some(10_000),
            drift_markets: None,
            drift_priority_fee_endpoint: get_drift_priority_fee_endpoint(DriftEnv::Devnet),
        };

        Self {
            name: config.bot_id,
            dry_run: config.dry_run,
            run_once: config.run_once.unwrap_or(false),
            default_interval_ms: 120000,
            drift_client,
            interval_ids: Vec::new(),
            priority_fee_subscriber_map: PriorityFeeSubscriberMap::new(priority_config),
            lookup_table_account: None,
            watchdog_timer_last_par_time: Instant::now(),
            in_progress: false,
        }
    }
}
