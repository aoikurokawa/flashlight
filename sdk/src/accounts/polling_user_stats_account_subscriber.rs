use std::sync::{Arc, Mutex};

use anchor_client::Program;
use log::warn;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tokio::sync::Mutex as TokioMutex;

use crate::{
    event_emitter::EventEmitter,
    types::{DataAndSlot, SdkError, SdkResult, UserStatsAccount},
};

use super::BulkAccountLoader;

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

    pub(crate) async fn add_to_account_loader(&mut self) {
        if self.callback_id.is_some() {
            return;
        }

        let user_stats = Arc::new(Mutex::new(self.user_stats.clone()));
        let user_stats_account_pubkey = self.user_stats_account_pubkey;
        let program = self.program.clone();
        let event_emitter = self.event_emitter.clone();

        self.callback_id = Some(
            self.account_loader
                .add_account(
                    user_stats_account_pubkey,
                    Arc::new(TokioMutex::new(move |buffer: Vec<u8>, slot: u64| {
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
                    })),
                )
                .await,
        );

        // self.account_loader.add_error_callback(|error: String| {
        //     self.event_emitter.emit("error", event)
        // }).await;
    }

    async fn fetch_if_unloaded(&mut self) -> SdkResult<()> {
        if self.user_stats.is_none() {
            self.fetch().await?;
        }

        Ok(())
    }

    fn does_account_exist(&self) -> bool {
        self.user_stats.is_some()
    }

    fn assert_is_subscribed(&self) -> SdkResult<()> {
        if !self.is_subscribed {
            return Err(SdkError::Generic(
                "You must call subscribe before using this function".to_string(),
            ));
        }

        Ok(())
    }

    pub(crate) async fn subscribe(
        &mut self,
        user_stats_account: Option<UserStatsAccount>,
    ) -> SdkResult<bool> {
        if self.is_subscribed {
            return Ok(true);
        }

        if let Some(user_stats_account) = user_stats_account {
            self.user_stats = Some(DataAndSlot {
                slot: 0,
                data: user_stats_account,
            });
        }

        Ok(false)
    }

    pub(crate) async fn fetch(&mut self) -> SdkResult<()> {
        let slot = self.program.rpc().get_slot()?;
        match self
            .program
            .account::<UserStatsAccount>(self.user_stats_account_pubkey)
        {
            Ok(account) => {
                if let Some(user_stats) = &self.user_stats {
                    if slot > user_stats.slot {
                        self.user_stats = Some(DataAndSlot {
                            slot,
                            data: account,
                        });
                    }
                }
            }
            Err(e) => {
                warn!(
                    "PollingUserStatsAccountSubscriber.fetch() UserStatsAccount does not exist: {}",
                    e
                );
            }
        }

        Ok(())
    }

    pub(crate) async fn unsubscribe(&mut self) {
        if !self.is_subscribed {
            return;
        }

        if let Some(callback_id) = &self.callback_id {
            self.account_loader
                .remove_account(self.user_stats_account_pubkey, callback_id.to_owned())
                .await;
            self.callback_id = None;
        }

        if let Some(error_callback_id) = &self.error_callback_id {
            self.account_loader
                .remove_error_callback(error_callback_id.to_owned())
                .await;
            self.error_callback_id = None;
        }

        self.is_subscribed = false;
    }

    pub(crate) fn get_user_stats_account_and_slot(
        &self,
    ) -> SdkResult<Option<DataAndSlot<UserStatsAccount>>> {
        if !self.does_account_exist() {
            return Err(SdkError::Generic(
                "You must subscribe or fetch before using this function".to_string(),
            ));
        }

        Ok(self.user_stats.clone())
    }
}
