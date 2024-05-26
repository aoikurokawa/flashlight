use solana_sdk::pubkey::Pubkey;

use crate::{
    accounts::UserStatsAccountSubscriber,
    user_stats_config::{UserStatsConfig, UserStatsSubscriptionConfig},
    AccountProvider, DriftClient,
};

pub struct UserStats<T: AccountProvider, U> {
    drift_client: DriftClient<T, U>,
    user_stats_account_pubkey: Pubkey,
    account_subscriber: Box<dyn UserStatsAccountSubscriber>,
    is_subscribed: bool,
}

impl<T: AccountProvider, U> UserStats<T, U> {
    pub fn new(config: UserStatsConfig<T, U>) -> Self {
        // let mut account_subscriber = Box::new();
        //         match config.account_subscription {
        //             UserStatsSubscriptionConfig::Polling { account_loader } => {
        //                 // Polling
        //             }
        //         }
        //
        // Self {
        //     drift_client: config.drift_client,
        //     user_stats_account_pubkey: config.user__stats_account_public_key,
        // }
        todo!()
    }
}
