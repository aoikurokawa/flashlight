use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

use crate::{
    accounts::{BulkAccountLoader, UserAccountSubscriber},
    drift_client::DriftClient,
    AccountProvider,
};

pub struct UserConfig<T>
where
    T: AccountProvider,
{
    pub account_subscription: Option<UserSubscriptionConfig>,
    pub drift_client: DriftClient<T>,
    pub user_account_public_key: Pubkey,
}

#[derive(Clone)]
pub enum UserSubscriptionConfig {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader,
    },
    Custom {
        user_account_subscriber: UserAccountSubscriber,
    },
}
