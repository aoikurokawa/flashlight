use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

use crate::{accounts::BulkAccountLoader, AccountProvider, DriftClient};

pub struct UserConfig<T, U>
where
    T: AccountProvider,
{
    account_subscription: Option<UserSubscriptionConfig<U>>,
    drift_client: DriftClient<T, U>,
    user_account_public_key: Pubkey,
}

#[derive(Clone)]
pub enum UserSubscriptionConfig<U> {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader,
    },
    Custom {
        user_account_subscriber: Box<U>,
    },
}
