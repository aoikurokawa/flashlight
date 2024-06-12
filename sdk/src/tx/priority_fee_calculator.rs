use std::time::{Duration, Instant};

use solana_sdk::{compute_budget::ComputeBudgetInstruction, instruction::Instruction};

/// This class determines whether a priority fee needs to be included in a transaction based on
/// a recent history of timed out transactions.
#[derive(Debug, Clone)]
pub struct PriorityFeeCalculator {
    last_tx_timeout_count: u64,
    priority_fee_triggered: bool,
    last_tx_timeout_count_triggered: Instant,
    priority_fee_latch_duration_ms: Duration,
}

impl PriorityFeeCalculator {
    /// Constructor for the PriorityFeeCalculator class.
    pub fn new(current_time_ms: Instant, priority_fee_latch_duration_ms: Option<Duration>) -> Self {
        let priority_fee_latch_duration_ms =
            priority_fee_latch_duration_ms.unwrap_or(Duration::from_millis(10 * 1000));
        Self {
            last_tx_timeout_count: 0,
            priority_fee_triggered: false,
            last_tx_timeout_count_triggered: current_time_ms,
            priority_fee_latch_duration_ms,
        }
    }

    /// Update the priority fee state based on the current time and the current timeout count.
    pub fn update_priority_fee(&mut self, current_time_ms: Instant, tx_time_count: u64) -> bool {
        let mut trigger_priority_fee = false;

        if tx_time_count > self.last_tx_timeout_count {
            self.last_tx_timeout_count = tx_time_count;
            self.last_tx_timeout_count_triggered = current_time_ms;
            trigger_priority_fee = true;
        } else {
            if !self.priority_fee_triggered {
                trigger_priority_fee = false;
            } else if current_time_ms - self.last_tx_timeout_count_triggered
                < self.priority_fee_latch_duration_ms
            {
                trigger_priority_fee = true;
            }
        }

        self.priority_fee_triggered = trigger_priority_fee;

        trigger_priority_fee
    }

    /// This method returns a transaction instruction list that sets the compute limit on the ComputeBudget program.
    pub fn generate_compute_budget_ixs(&self, compute_unit_limit: u32) -> Vec<Instruction> {
        let ix = ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit);
        vec![ix]
    }

    /// Calculates the compute unit price to use based on the desired additional fee to pay and the compute unit limit.
    pub fn calculate_compute_unit_price(
        &self,
        compute_unit_limit: u32,
        additional_fe_micro_lamports: u32,
    ) -> u32 {
        additional_fe_micro_lamports / compute_unit_limit
    }

    /// This method generates a list of transaction instructions for the ComputeBudget program, and includes a priority fee if it's required
    pub fn generate_compute_budget_with_priority_fee_ix(
        &self,
        compute_unit_limit: u32,
        use_priority_fee: bool,
        additional_fe_micro_lamports: u32,
    ) -> Vec<Instruction> {
        let mut ixs = self.generate_compute_budget_ixs(compute_unit_limit);

        if use_priority_fee {
            let compute_unit_price =
                self.calculate_compute_unit_price(compute_unit_limit, additional_fe_micro_lamports);
            ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
                compute_unit_price as u64,
            ));
        }

        ixs
    }
}
