use super::types::{PriorityFeeResponse, PriorityFeeStrategy};

#[derive(Debug, Clone)]
pub(crate) struct AverageOverSlotsStrategy;

impl PriorityFeeStrategy for AverageOverSlotsStrategy {
    fn calculate(&self, samples: PriorityFeeResponse) -> u64 {
        if let PriorityFeeResponse::Solana(res) = samples {
            if res.is_empty() {
                return 0;
            }

            let running_sum_fees: u64 = res.iter().map(|x| x.prioritization_fee).sum();

            return running_sum_fees / res.len() as u64;
        }

        0
    }
}
