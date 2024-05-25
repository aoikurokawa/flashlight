use std::collections::HashMap;

use drift::state::user::UserStats;

use crate::{accounts::BulkAccountLoader, AccountProvider, DriftClient};

pub struct UserStatsMap<T, F, E>
where
    T: AccountProvider,
{
    user_stats_map: HashMap<String, UserStats>,
    drift_client: DriftClient<T, F, E>,
    //    bulk_account_provider: BulkAccountLoader,
}
