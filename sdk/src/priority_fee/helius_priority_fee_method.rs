use std::collections::HashMap;

pub(crate) enum HeliusPriorityLevel {
    /// 25th percentile
    MIN,
    /// 25th percentile
    LOW,
    /// 50th percentile
    MEDIUM,
    /// 75th percentile
    HIGH,
    /// 95th percentile
    VERYHIGH,
    /// 100th percentile
    UNSAFEMAX,
}

impl From<&str> for HeliusPriorityLevel {
    fn from(value: &str) -> Self {
        match value {
            "min" => HeliusPriorityLevel::MIN,
            "low" => HeliusPriorityLevel::LOW,
            "medium" => HeliusPriorityLevel::MEDIUM,
            "high" => HeliusPriorityLevel::HIGH,
            "veryHigh" => HeliusPriorityLevel::VERYHIGH,
            "unsafeMax" => HeliusPriorityLevel::UNSAFEMAX,
            val => panic!("Invalid string for HeliusPriorityLevel: {val}"),
        }
    }
}

pub(crate) struct HeliusPriorityFeeLevels(HashMap<HeliusPriorityLevel, u64>);

struct HeliusPriorityFeeResult {
    priority_fee_estimate: Option<u64>,
    priority_fee_levels: Option<HeliusPriorityFeeLevels>,
}

pub(crate) struct HeliusPriorityFeeResponse {
    jsonrpc: String,
    result: HeliusPriorityFeeResult,
    id: String,
}
