use crate::priority_fee::types::DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS;

use super::{
    average_over_slots_strategy::AverageOverSlotsStrategy,
    drift_priority_fee_method::DriftMarketInfo,
    helius_priority_fee_method::HeliusPriorityFeeLevels,
    max_over_slots_strategy::MaxOverSlotsStrategy,
    types::{PriorityFeeMethod, PriorityFeeStrategy, PriorityFeeSubscriberConfig},
};

pub struct PriorityFeeSubscriber {
    // connection: Connection,
    frequency_ms: u64,
    addresses: Vec<String>,
    drift_markets: Option<Vec<DriftMarketInfo>>,
    custom_strategy: Option<Box<dyn PriorityFeeStrategy>>,
    average_strategy: AverageOverSlotsStrategy,
    max_strategy: MaxOverSlotsStrategy,
    priority_fee_method: PriorityFeeMethod,
    lookback_distance: u64,
    max_fee_micro_lamports: Option<u64>,
    priority_fee_multiplier: Option<u64>,

    drift_priority_fee_endpoint: Option<String>,
    helius_rpc_url: Option<String>,
    last_helius_sample: Option<HeliusPriorityFeeLevels>,

    // interval_id?: ReturnType<typeof setTimeout>;
    latest_priority_fee: u64,
    last_custom_strategy_result: u64,
    last_avg_strategy_result: u64,
    last_max_strategy_result: u64,
    last_slot_seen: u64,
}

impl PriorityFeeSubscriber {
    pub fn new(config: PriorityFeeSubscriberConfig) -> Self {
        let frequency_ms = match config.frequency_ms {
            Some(ms) => ms,
            None => DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS,
        };

        let addresses = match config.addresses {
            Some(keys) => keys.iter().map(|key| key.to_string()).collect(),
            None => vec![],
        };

        let average_strategy = AverageOverSlotsStrategy;
        let custom_strategy = match config.custom_strategy {
            Some(strategy) => strategy,
            None => Box::new(average_strategy),
        };

        let lookback_distance = match config.slots_to_check {
            Some(x) => x,
            None => 50,
        };

        let mut priority_fee_method = None;
        let mut helius_rpc_url = None;
        let mut drift_priority_fee_endpoint = None;
        if let Some(priority_fee_method) = config.priority_fee_method {
            priority_fee_method = priority_fee_method;

            if priority_fee_method == PriorityFeeMethod::Helius {
                match config.helius_rpc_url {
                    Some(rpc) => {
                        helius_rpc_url = Some(rpc);
                    }
                    None => {}
                }
            } else if priority_fee_method == PriorityFeeMethod::Drift {
                drift_priority_fee_endpoint = config.drift_priority_fee_endpoint;
            }
        }

        Self {
            frequency_ms,
            addresses,
            drift_markets: config.drift_markets,
            custom_strategy: Some(custom_strategy),
            average_strategy,
            max_strategy: (),
            priority_fee_method: (),
            lookback_distance,
            max_fee_micro_lamports: (),
            priority_fee_multiplier: (),
            drift_priority_fee_endpoint,
            helius_rpc_url,
            last_helius_sample: (),
            latest_priority_fee: (),
            last_custom_strategy_result: (),
            last_avg_strategy_result: (),
            last_max_strategy_result: (),
            last_slot_seen: (),
        }
    }
}
