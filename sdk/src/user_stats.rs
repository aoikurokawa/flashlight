use std::sync::Arc;

use anchor_client::{Client, Cluster, Program};
pub use drift::ID as PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;

use crate::{
    accounts::{
        polling_user_stats_account_subscriber::PollingUserStatsAccountSubscriber,
        web_socket_user_stats_account_subscriber::WebSocketUserStatsAccountSubscriber, ResubOpts,
        UserStatsAccountSubscriber,
    },
    types::{SdkError, SdkResult},
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
    pub fn new(config: UserStatsConfig<T, U>) -> SdkResult<Self> {
        let client = Client::new(Cluster::Devnet, config.drift_client.wallet().signer.clone());

        let account_subscriber: Box<dyn UserStatsAccountSubscriber> =
            match config.account_subscription {
                Some(account_sub) => match account_sub {
                    UserStatsSubscriptionConfig::Polling { account_loader } => {
                        Box::new(PollingUserStatsAccountSubscriber::new(
                            Arc::new(client.program(PROGRAM_ID)),
                            config.user_stats_account_public_key,
                            account_loader,
                        ))
                    }
                    UserStatsSubscriptionConfig::WebSocket {
                        resub_timeout_ms,
                        log_resub_messages,
                        commitment,
                    } => Box::new(WebSocketUserStatsAccountSubscriber::new(
                        client.program(PROGRAM_ID),
                        config.user_stats_account_public_key,
                        Some(ResubOpts {
                            resub_timeout_ms,
                            log_resub_messages,
                        }),
                        commitment,
                    )),
                    UserStatsSubscriptionConfig::Custom => {
                        return Err(SdkError::Generic(format!(
                            "Unknown user stats account subscription type"
                        )));
                    }
                },
                None => {
                    return Err(SdkError::Generic(format!(
                        "Unknown user stats account subscription type"
                    )));
                }
            };

        Ok(Self {
            drift_client: config.drift_client,
            user_stats_account_pubkey: config.user_stats_account_public_key,
            account_subscriber,
            is_subscribed: false,
        })
    }
}
