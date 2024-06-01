use sdk::{DriftClient, AccountProvider};

pub struct FundingRateUpdaterBot<'a, T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    run_once: bool,
    default_interval_ms: u64,
    drift_client: &'a DriftClient<T, U>,
    interval_ids: Vec<u64>,
    // priority_fee_subscriber_map: Priority
}
