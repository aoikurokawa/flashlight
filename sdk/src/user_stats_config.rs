use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

use crate::{accounts::BulkAccountLoader, AccountProvider, DriftClient};

pub struct UserStatsConfig<'a, T, U>
where
    T: AccountProvider,
{
    pub account_subscription: Option<UserStatsSubscriptionConfig>,
    pub drift_client: &'a DriftClient<T, U>,
    pub user_stats_account_public_key: Pubkey,
}

#[derive(Clone)]
pub enum UserStatsSubscriptionConfig {
    WebSocket {
        resub_timeout_ms: Option<u64>,
        log_resub_messages: Option<bool>,
        commitment: Option<CommitmentConfig>,
    },
    Polling {
        account_loader: BulkAccountLoader,
    },
    Custom,
}
