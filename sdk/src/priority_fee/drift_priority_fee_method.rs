use std::collections::HashMap;

use drift::state::user::MarketType;

use super::helius_priority_fee_method::HeliusPriorityLevel;

pub(crate) struct DriftMarketInfo {
    market_type: String,
    market_index: u16,
}

pub(crate) struct DriftPriorityFeeLevels {
    priority_fee_level: HashMap<HeliusPriorityLevel, u64>,
    market_type: MarketType,
    market_index: u64,
}

pub(crate) struct DriftPriorityFeeResponse(Vec<DriftPriorityFeeLevels>);
