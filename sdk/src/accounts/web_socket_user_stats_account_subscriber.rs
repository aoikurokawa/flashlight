use std::sync::Arc;

use anchor_client::Program;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

use crate::{
    event_emitter::EventEmitter,
    types::{DataAndSlot, SdkResult, UserStatsAccount},
    WebsocketAccountSubscriber,
};

use super::ResubOpts;

#[allow(dead_code)]
pub struct WebSocketUserStatsAccountSubscriber {
    is_subscribed: bool,
    resub_opts: Option<ResubOpts>,
    commitment: Option<CommitmentConfig>,
    program: Program<Arc<Keypair>>,
    event_emitter: EventEmitter,
    user_stats_account_pubkey: Pubkey,
    user_stats_account_subscriber: WebsocketAccountSubscriber<UserStatsAccount>,
}

impl WebSocketUserStatsAccountSubscriber {
    pub fn new(
        program: Program<Arc<Keypair>>,
        user_stats_account_pubkey: Pubkey,
        resub_opts: Option<ResubOpts>,
        commitment: Option<CommitmentConfig>,
    ) -> Self {
        let user_stats_account_subscriber = WebsocketAccountSubscriber::new(
            "user_stats",
            &program.rpc().url(),
            user_stats_account_pubkey,
            commitment.unwrap(),
            EventEmitter::new(),
        );

        Self {
            is_subscribed: false,
            program,
            user_stats_account_pubkey,
            event_emitter: EventEmitter::new(),
            resub_opts,
            commitment,
            user_stats_account_subscriber,
        }
    }

    pub(crate) async fn subscribe(
        &mut self,
        user_stats_account: Option<UserStatsAccount>,
    ) -> SdkResult<bool> {
        if self.is_subscribed {
            return Ok(true);
        }

        // if let Some(user_stats_account) = user_stats_account {
        // self.user_stats_account_subscriber.subscribe(|data| {
        //     self.event_emitter.emit("user_stats_account_update", data);
        //     // self.event_emitter.emit("update", data);
        // });

        // self.user_stats_account_subscriber.subscribe();
        // self.event_emitter.emit("update", event);
        // self.is_subscribed = true;
        // }

        if user_stats_account.is_some() {
            self.user_stats_account_subscriber.subscribe().await?;
            self.is_subscribed = true;
        }

        Ok(true)
    }

    pub(crate) async fn fetch(&mut self) -> SdkResult<()> {
        self.user_stats_account_subscriber.fetch().await?;
        Ok(())
    }

    pub(crate) async fn unsubscribe(&mut self) -> SdkResult<()> {
        if !self.is_subscribed {
            return Ok(());
        }

        self.user_stats_account_subscriber.unsubscribe().await?;

        self.is_subscribed = false;

        Ok(())
    }

    pub(crate) fn get_user_stats_account_and_slot(
        &self,
    ) -> SdkResult<Option<DataAndSlot<UserStatsAccount>>> {
        assert!(
            self.is_subscribed,
            "You must call subscribe before using this function"
        );

        log::debug!("get_user_stats_account_and_slot");

        let data_and_slot = self
            .user_stats_account_subscriber
            .data_and_slot
            .lock()
            .unwrap();

        Ok(data_and_slot.clone())
    }
}
