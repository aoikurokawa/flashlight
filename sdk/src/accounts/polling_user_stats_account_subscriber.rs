use std::sync::{Arc, Mutex};

use anchor_client::Program;
use async_trait::async_trait;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{
    event_emitter::EventEmitter,
    types::{DataAndSlot, UserStatsAccount},
};

use super::{BulkAccountLoader, UserStatsAccountSubscriber};

pub struct PollingUserStatsAccountSubscriber {
    is_subscribed: bool,
    program: Arc<Program<Arc<Keypair>>>,
    event_emitter: EventEmitter,
    user_stats_account_pubkey: Pubkey,
    account_loader: BulkAccountLoader,
    callback_id: Option<String>,
    error_callback_id: Option<String>,
    user_stats: Option<DataAndSlot<UserStatsAccount>>,
}

impl PollingUserStatsAccountSubscriber {
    pub fn new(
        program: Arc<Program<Arc<Keypair>>>,
        user_stats_account_pubkey: Pubkey,
        account_loader: BulkAccountLoader,
    ) -> Self {
        Self {
            is_subscribed: false,
            program,
            event_emitter: EventEmitter::new(),
            user_stats_account_pubkey,
            account_loader,
            callback_id: None,
            error_callback_id: None,
            user_stats: None,
        }
    }

    async fn add_to_account_loader(&mut self) {
        if self.callback_id.is_some() {
            return;
        }

        let user_stats = Arc::new(Mutex::new(self.user_stats.clone()));
        let user_stats_account_pubkey = self.user_stats_account_pubkey;
        let program = self.program.clone();
        let event_emitter = self.event_emitter.clone();

        self.account_loader.add_account(
            user_stats_account_pubkey,
            Arc::new(move |buffer: Vec<u8>, slot: u64| {
                if buffer.is_empty() {
                    return;
                }

                let mut user_stats = user_stats.lock().unwrap();
                if let Some(user_stats) = &*user_stats {
                    if user_stats.slot > slot {
                        return;
                    }
                }

                // let pubkey = Pubkey::new_from_array(&buffer[..]);
                let mut array = [0u8; 32];
                array.copy_from_slice(&buffer);
                let account: UserStatsAccount =
                    program.account(Pubkey::new_from_array(array)).unwrap();

                *user_stats = Some(DataAndSlot {
                    slot,
                    data: account,
                });
                event_emitter.emit("user_stats_account_update", Box::new(account));
            }),
        );
    }
}

#[async_trait]
impl UserStatsAccountSubscriber for PollingUserStatsAccountSubscriber {
    async fn subscribe(&mut self, user_stats_account: Option<UserStatsAccount>) -> bool {
        if self.is_subscribed {
            return true;
        }

        if let Some(user_stats_account) = user_stats_account {
            self.user_stats = Some(DataAndSlot {
                slot: 0,
                data: user_stats_account,
            });
        }

        false
    }

    async fn fetch(&self) {}
    async fn unsubscribe(&mut self) {}
    fn get_user_account_and_slot(&self) -> DataAndSlot<UserStatsAccount> {
        todo!()
    }
}
