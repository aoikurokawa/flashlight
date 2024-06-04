pub struct PriorityFeeCalculator {
    last_tx_timeout_count: u64,
    priority_fee_triggered: bool,
    last_tx_timeout_count_triggered: u64,
    priority_fee_latch_duration_ms: u64,
}

impl PriorityFeeCalculator {
}
