use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

use crate::{accounts::BulkAccountLoader, AccountProvider, DriftClient};

pub struct UserConfig<T, F, Fut, G, C>
where
    C: AccountProvider,
{
    account_subscription: Option<UserSubscriptionConfig<T, F, Fut, G>>,
    drift_client: DriftClient<C>,
    user_account_public_key: Pubkey,
}

pub enum UserSubscriptionConfig<T, F, Fut, G> {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader<F, Fut, G>,
    },
    Custom {
        user_account_subscriber: Box<T>,
    },
}
