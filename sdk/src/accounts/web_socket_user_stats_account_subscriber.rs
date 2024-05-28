use std::sync::Arc;

use anchor_client::Program;
use async_trait::async_trait;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

use crate::{
    event_emitter::{self, EventEmitter},
    types::{DataAndSlot, SdkResult, UserStatsAccount},
    WebsocketAccountSubscriber,
};

use super::{AccountSubscriber, ResubOpts, UserStatsAccountSubscriber};

pub struct WebSocketUserStatsAccountSubscriber<T, AS: AccountSubscriber<T>> {
    is_subscribed: bool,
    resub_opts: Option<ResubOpts>,
    commitment: Option<CommitmentConfig>,
    program: Program<Arc<Keypair>>,
    event_emitter: EventEmitter,
    user_stats_account_pubkey: Pubkey,
    user_stats_account_subscriber: AS,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, AS: AccountSubscriber<T>> WebSocketUserStatsAccountSubscriber<T, AS> {
    pub fn new(
        program: Program<Arc<Keypair>>,
        user_stats_account_pubkey: Pubkey,
        resub_opts: Option<ResubOpts>,
        commitment: Option<CommitmentConfig>,
    ) -> Self {
        let user_stats_account_subscriber: WebsocketAccountSubscriber<UserStatsAccount> =
            WebsocketAccountSubscriber::new(
                "userStats",
                program.rpc().url(),
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
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T: std::marker::Send, AS: AccountSubscriber<T> + std::marker::Send> UserStatsAccountSubscriber
    for WebSocketUserStatsAccountSubscriber<T, AS>
{
    async fn subscribe(&mut self, user_stats_account: Option<UserStatsAccount>) -> bool {
        if self.is_subscribed {
            return true;
        }

        false
    }

    async fn fetch(&mut self) -> SdkResult<()> {
        Ok(())
    }

    async fn unsubscribe(&mut self) {}

    fn get_user_account_and_slot(&self) -> SdkResult<Option<DataAndSlot<UserStatsAccount>>> {
        todo!()
    }
}
