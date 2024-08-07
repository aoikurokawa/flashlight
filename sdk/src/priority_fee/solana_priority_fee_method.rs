use solana_sdk::pubkey::Pubkey;

use crate::{drift_client::DriftClient, types::SdkResult, AccountProvider};

pub struct SolanaPriorityFeeResponse {
    pub(crate) slot: u64,
    pub(crate) prioritization_fee: u64,
}

pub(crate) async fn fetch_solana_priority_fee<T: AccountProvider>(
    drift_client: &DriftClient<T>,
    lookback_distance: u8,
    addresses: &[Pubkey],
) -> SdkResult<Vec<SolanaPriorityFeeResponse>> {
    let mut results: Vec<SolanaPriorityFeeResponse> = drift_client
        .backend
        .rpc_client
        .get_recent_prioritization_fees(addresses)
        .await?
        .iter()
        .map(|res| SolanaPriorityFeeResponse {
            slot: res.slot,
            prioritization_fee: res.prioritization_fee,
        })
        .collect();

    if results.is_empty() {
        return Ok(vec![]);
    }

    results.sort_by(|a, b| b.slot.cmp(&a.slot));
    let cutoff_slot = results[0].slot - lookback_distance as u64;

    Ok(results
        .into_iter()
        .filter(|x| x.slot >= cutoff_slot)
        .collect())
}
