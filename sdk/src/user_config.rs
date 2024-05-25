use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

use crate::{accounts::BulkAccountLoader, AccountProvider, DriftClient};

pub struct UserConfig<T, F, E>
where
    T: AccountProvider,
{
    account_subscription: Option<UserSubscriptionConfig<T, F, E>>,
    drift_client: DriftClient<T, F, E>,
    user_account_public_key: Pubkey,
}

pub enum UserSubscriptionConfig<T, F, E> {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader<F, E>,
    },
    Custom {
        user_account_subscriber: Box<T>,
    },
}
