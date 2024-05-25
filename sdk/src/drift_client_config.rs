use std::sync::Arc;

use solana_sdk::commitment_config::CommitmentLevel;

use crate::{accounts::BulkAccountLoader, user_config::UserSubscriptionConfig};

#[derive(Clone)]
pub struct ClientOpts<F, E> {
    account_subscription: Option<DriftClientSubscriptionConfig<F, E>>,
    active_sub_account_id: u16,
    sub_account_ids: Vec<u16>,
}

impl<F, E> Default for ClientOpts<F, E> {
    fn default() -> Self {
        Self {
            account_subscription: None,
            active_sub_account_id: 0,
            sub_account_ids: vec![0],
        }
    }
}

impl<F, E> ClientOpts<F, E> {
    pub fn new(
        active_sub_account_id: u16,
        sub_account_ids: Option<Vec<u16>>,
        account_subscription: Option<DriftClientSubscriptionConfig<F, E>>,
    ) -> Self {
        let sub_account_ids = sub_account_ids.unwrap_or(vec![active_sub_account_id]);

        Self {
            account_subscription,
            active_sub_account_id,
            sub_account_ids,
        }
    }

    pub fn active_sub_account_id(&self) -> u16 {
        self.active_sub_account_id
    }

    pub fn sub_account_ids(self) -> Vec<u16> {
        self.sub_account_ids
    }

    pub fn account_subscription(&self) -> Option<DriftClientSubscriptionConfig<F, E>> {
        match self.account_subscription {
            DriftClientSubscriptionConfig::WebSocket {
                resub_timeout_ms,
                log_resub_messages,
                commitment,
            } => UserSubscriptionConfig::WebSocket,
            DriftClientSubscriptionConfig::Polling { account_loader } => {
                UserSubscriptionConfig::Polling { account_loader: () }
            }
        }
    }
}

pub enum DriftClientSubscriptionConfig<F, E>
where
    F: Fn(Vec<u8>, u64) + Send + Sync + 'static,
    E: Fn(Arc<dyn std::error::Error + Send + Sync>) + Send + Sync + 'static,
{
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader<F, E>,
    },
}
