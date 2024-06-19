use std::sync::Arc;

use anchor_client::{Client, Cluster};
pub use drift::ID as PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;

use crate::{
    accounts::{
        polling_user_stats_account_subscriber::PollingUserStatsAccountSubscriber,
        web_socket_user_stats_account_subscriber::WebSocketUserStatsAccountSubscriber, ResubOpts,
        UserStatsAccountSubscriber,
    },
    addresses::pda::{get_user_account_pubkey, get_user_stats_account_pubkey},
    drift_client::DriftClient,
    types::{DataAndSlot, ReferrerInfo, SdkError, SdkResult, UserStatsAccount},
    user_stats_config::{UserStatsConfig, UserStatsSubscriptionConfig},
    AccountProvider,
};

pub struct UserStats<T: AccountProvider> {
    pub drift_client: Arc<DriftClient<T>>,
    pub user_stats_account_pubkey: Pubkey,
    pub account_subscriber: UserStatsAccountSubscriber,
    pub is_subscribed: bool,
}

impl<T: AccountProvider> UserStats<T> {
    pub fn new(config: UserStatsConfig<T>) -> SdkResult<Self> {
        let client = Client::new(Cluster::Devnet, config.drift_client.wallet().signer.clone());

        let account_subscriber = match config.account_subscription {
            Some(account_sub) => match account_sub {
                UserStatsSubscriptionConfig::Polling { account_loader } => {
                    UserStatsAccountSubscriber::Polling(PollingUserStatsAccountSubscriber::new(
                        Arc::new(client.program(PROGRAM_ID)),
                        config.user_stats_account_public_key,
                        account_loader,
                    ))
                }
                UserStatsSubscriptionConfig::WebSocket {
                    resub_timeout_ms,
                    log_resub_messages,
                    commitment,
                } => {
                    UserStatsAccountSubscriber::WebSocket(WebSocketUserStatsAccountSubscriber::new(
                        client.program(PROGRAM_ID),
                        config.user_stats_account_public_key,
                        Some(ResubOpts {
                            resub_timeout_ms,
                            log_resub_messages,
                        }),
                        commitment,
                    ))
                }
                UserStatsSubscriptionConfig::Custom => {
                    return Err(SdkError::Generic(
                        "Unknown user stats account subscription type".to_string(),
                    ));
                }
            },
            None => {
                return Err(SdkError::Generic(
                    "Unknown user stats account subscription type".to_string(),
                ));
            }
        };

        Ok(Self {
            drift_client: config.drift_client,
            user_stats_account_pubkey: config.user_stats_account_public_key,
            account_subscriber,
            is_subscribed: false,
        })
    }

    pub async fn subscribe(
        &mut self,
        user_stats_account: Option<UserStatsAccount>,
    ) -> SdkResult<bool> {
        self.is_subscribed = self
            .account_subscriber
            .subscribe(user_stats_account)
            .await?;

        Ok(self.is_subscribed)
    }

    pub async fn fetch_accounts(&mut self) -> SdkResult<()> {
        self.account_subscriber.fetch().await?;

        Ok(())
    }

    pub async fn unsubscribe(&mut self) {
        self.account_subscriber.unsubscribe().await;
        self.is_subscribed = false;
    }

    pub fn get_account_and_slot(&self) -> SdkResult<Option<DataAndSlot<UserStatsAccount>>> {
        let user_stats_account = self.account_subscriber.get_user_stats_account_and_slot()?;
        Ok(user_stats_account)
    }

    pub fn get_account(&self) -> SdkResult<Option<UserStatsAccount>> {
        let account_and_slot = self.account_subscriber.get_user_stats_account_and_slot()?;

        if let Some(account) = account_and_slot {
            return Ok(Some(account.data));
        }

        Ok(None)
    }

    pub fn get_referrer_info(&self) -> SdkResult<Option<ReferrerInfo>> {
        let account = self.get_account()?;

        match account {
            Some(account) => {
                if account.referrer.eq(&Pubkey::default()) {
                    Ok(None)
                } else {
                    Ok(Some(ReferrerInfo {
                        referrer: get_user_account_pubkey(&PROGRAM_ID, account.referrer, Some(0)),
                        referrer_stats: get_user_stats_account_pubkey(
                            &PROGRAM_ID,
                            account.referrer,
                        ),
                    }))
                }
            }
            None => Ok(None),
        }
    }

    pub fn get_oldest_action_ts(account: UserStatsAccount) -> i64 {
        std::cmp::min(
            account.last_filler_volume_30d_ts,
            std::cmp::min(
                account.last_maker_volume_30d_ts,
                account.last_taker_volume_30d_ts,
            ),
        )
    }
}
