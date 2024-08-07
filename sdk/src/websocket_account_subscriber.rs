use std::sync::{Arc, Mutex};

use anchor_lang::AccountDeserialize;
use futures_util::StreamExt;
use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_client::{nonblocking::pubsub_client::PubsubClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

use crate::{
    error::SdkError,
    event_emitter::{Event, EventEmitter},
    types::DataAndSlot,
    utils::decode,
    SdkResult,
};

#[derive(Clone, Debug)]
pub(crate) struct AccountUpdate {
    pub pubkey: String,
    pub data: UiAccount,
    pub slot: u64,
}

impl Event for AccountUpdate {
    fn box_clone(&self) -> Box<dyn Event> {
        Box::new((*self).clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Clone)]
pub struct WebsocketAccountSubscriber<T>
where
    T: AccountDeserialize,
{
    subscription_name: &'static str,

    url: String,

    pubkey: Pubkey,

    pub(crate) commitment: CommitmentConfig,

    pub subscribed: bool,

    pub event_emitter: EventEmitter,

    unsubscriber: Option<tokio::sync::mpsc::Sender<()>>,

    pub(crate) data_and_slot: Arc<Mutex<Option<DataAndSlot<T>>>>,
}

impl<T> WebsocketAccountSubscriber<T>
where
    T: AccountDeserialize + Send + 'static,
{
    pub fn new(
        subscription_name: &'static str,
        url: &str,
        pubkey: Pubkey,
        commitment: CommitmentConfig,
        event_emitter: EventEmitter,
    ) -> Self {
        WebsocketAccountSubscriber {
            subscription_name,
            url: url.to_string(),
            pubkey,
            commitment,
            subscribed: false,
            event_emitter,
            unsubscriber: None,
            data_and_slot: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        if self.subscribed {
            return Ok(());
        }

        self.subscribed = true;
        self.subscribe_ws().await?;
        Ok(())
    }

    async fn subscribe_ws(&mut self) -> SdkResult<()> {
        let account_config = RpcAccountInfoConfig {
            commitment: Some(self.commitment),
            encoding: Some(UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        };
        let (unsub_tx, mut unsub_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.unsubscriber = Some(unsub_tx);

        let mut attempt = 0;
        let max_reconnection_attempts = 20;
        let base_delay = tokio::time::Duration::from_secs(2);

        let url = self.url.clone();
        let data_and_slot = self.data_and_slot.clone();

        tokio::spawn({
            let event_emitter = self.event_emitter.clone();
            let mut latest_slot = 0;
            let subscription_name = self.subscription_name;
            let pubkey = self.pubkey;
            async move {
                loop {
                    let pubsub = PubsubClient::new(&url).await?;

                    match pubsub
                        .account_subscribe(&pubkey, Some(account_config.clone()))
                        .await
                    {
                        Ok((mut account_updates, account_unsubscribe)) => loop {
                            attempt = 0;
                            tokio::select! {
                                message = account_updates.next() => {
                                    match message {
                                        Some(message) => {
                                            log::error!("Got message");
                                            let slot = message.context.slot;
                                            if slot >= latest_slot {
                                                latest_slot = slot;
                                                let account_update = AccountUpdate {
                                                    pubkey: pubkey.to_string(),
                                                    data: message.value.clone(),
                                                    slot,
                                                };
                                                event_emitter.emit(subscription_name, Box::new(account_update));
                                                let new_data = decode::<T>(message.value.data.clone()).expect("valid state data");
                                                let mut data_and_slot = data_and_slot.lock().unwrap();
                                                *data_and_slot = Some(DataAndSlot {slot, data: new_data } );
                                                drop(data_and_slot);
                                            }
                                        }
                                        None => {
                                            log::warn!("{subscription_name}: Account stream interrupted");
                                            account_unsubscribe().await;
                                            break;
                                        }
                                    }
                                }
                                unsub = unsub_rx.recv() => {
                                    if unsub.is_some() {
                                        log::debug!("{subscription_name}: Unsubscribing from account stream");
                                        account_unsubscribe().await;
                                        return Ok(());

                                    }
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("{subscription_name}: Failed to subscribe to account stream, retrying: {e}");
                            attempt += 1;
                            log::info!("Number of attempt: {attempt}");
                            if attempt >= max_reconnection_attempts {
                                log::error!("Max reconnection attempts {attempt} reached.");
                                return Err(SdkError::MaxReconnectionAttemptsReached);
                            }
                        }
                    }

                    if attempt >= max_reconnection_attempts {
                        log::error!("{subscription_name}: Max reconnection attempts reached");
                        return Err(SdkError::MaxReconnectionAttemptsReached);
                    }

                    let delay_duration = base_delay * 2_u32.pow(attempt);
                    log::debug!("{subscription_name}: Reconnecting in {delay_duration:?}");
                    tokio::time::sleep(delay_duration).await;
                    attempt += 1;
                }
            }
        });
        Ok(())
    }

    pub async fn unsubscribe(&mut self) -> SdkResult<()> {
        if self.subscribed && self.unsubscriber.is_some() {
            if let Err(e) = self.unsubscriber.as_ref().unwrap().send(()).await {
                log::error!("Failed to send unsubscribe signal: {e:?}");
                return Err(SdkError::CouldntUnsubscribe(e));
            }
            self.subscribed = false;
        }
        Ok(())
    }

    pub async fn fetch(&mut self) -> SdkResult<()> {
        Ok(())
    }
}
