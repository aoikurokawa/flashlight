use solana_sdk::pubkey::Pubkey;

use crate::{DriftClient, AccountProvider, accounts::UserStatsAccountSubscriber};

pub struct UserStats<T: AccountProvider, U> {
    drift_client: DriftClient<T, U>,
    user_stats_account_pubkey: Pubkey,
    account_subscriber: Box<dyn UserStatsAccountSubscriber>,
    is_subscribed: bool,
}

impl<T, U> UserStats<T, U> {
    pub fn new(
}
