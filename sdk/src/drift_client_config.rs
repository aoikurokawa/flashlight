use solana_sdk::commitment_config::CommitmentLevel;

use crate::{accounts::BulkAccountLoader, user_config::UserSubscriptionConfig};

#[derive(Clone)]
pub struct ClientOpts {
    account_subscription: Option<DriftClientSubscriptionConfig>,
    active_sub_account_id: u16,
    sub_account_ids: Vec<u16>,
}

impl Default for ClientOpts {
    fn default() -> Self {
        Self {
            account_subscription: None,
            active_sub_account_id: 0,
            sub_account_ids: vec![0],
        }
    }
}

impl ClientOpts {
    pub fn new(
        active_sub_account_id: u16,
        sub_account_ids: Option<Vec<u16>>,
        account_subscription: Option<DriftClientSubscriptionConfig>,
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

    pub fn sub_account_ids(&self) -> &[u16] {
        &self.sub_account_ids
    }

    pub fn account_subscription(&self) -> Option<UserSubscriptionConfig> {
        match &self.account_subscription {
            Some(subscription) => match subscription {
                DriftClientSubscriptionConfig::WebSocket {
                    resub_timeout_ms,
                    log_resub_messages,
                    commitment,
                } => Some(UserSubscriptionConfig::WebSocket {
                    resub_timeout_ms: *resub_timeout_ms,
                    log_resub_messages: *log_resub_messages,
                    commitment: *commitment,
                }),
                DriftClientSubscriptionConfig::Polling { account_loader } => {
                    Some(UserSubscriptionConfig::Polling {
                        account_loader: account_loader.clone(),
                    })
                }
            },
            None => None,
        }
    }
}

#[derive(Clone)]
pub enum DriftClientSubscriptionConfig {
    WebSocket {
        resub_timeout_ms: u16,
        log_resub_messages: bool,
        commitment: CommitmentLevel,
    },
    Polling {
        account_loader: BulkAccountLoader,
    },
}
