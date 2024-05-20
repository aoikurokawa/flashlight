use super::types::{PriorityFeeResponse, PriorityFeeStrategy};

pub(crate) struct MaxOverSlotsStrategy;

impl PriorityFeeStrategy for MaxOverSlotsStrategy {
    fn calculate(&self, samples: PriorityFeeResponse) -> u64 {
        if let PriorityFeeResponse::Solana(res) = samples {
            if res.is_empty() {
                return 0;
            }

            return res.iter().map(|x| x.prioritization_fee).max().unwrap();
        }

        0
    }
}
