use solana_sdk::commitment_config::CommitmentLevel;

use crate::BulkAccountLoader;

pub enum UserSubscriptionConfig {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        accountLoader: BulkAccountLoader,
    },
    Custom {
        user_account_subscriber: UserAccountSubscriber,
    },
}
