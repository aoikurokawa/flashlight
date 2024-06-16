use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::StreamExt;
use solana_account_decoder::UiAccountData;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::{
    clock::Clock,
    commitment_config::CommitmentLevel,
    sysvar::{self},
};
use tokio::{sync::Mutex, time::Duration};

use crate::{
    event_emitter::EventEmitter,
    types::{SdkError, SdkResult},
};

use super::clock_subscriber_event::ClockSubscriberEvent;

pub struct ClockSubscriberConfig {
    commitment: CommitmentLevel,
    resub_timeout_ms: Option<u64>,
}

#[derive(Debug)]
pub struct ClockSubscriber {
    rpc_client: Arc<PubsubClient>,
    latest_slot: Arc<Mutex<u64>>,
    current_ts: Arc<Mutex<i64>>,
    _subscription_id: u64,
    _commitment: CommitmentLevel,
    event_emitter: Arc<EventEmitter>,
    // timeout_id: Option<>
    resub_timeout_ms: Option<u64>,
    is_unsubscribing: Arc<Mutex<bool>>,
    receiving_data: Arc<Mutex<bool>>,
    subscription_task: Option<Arc<Mutex<tokio::task::JoinHandle<()>>>>,
    shutdown_sender: Option<Arc<Mutex<tokio::sync::mpsc::Sender<()>>>>,
}

impl ClockSubscriber {
    pub fn new(rpc_client: Arc<PubsubClient>, config: Option<ClockSubscriberConfig>) -> Self {
        let mut commitment = CommitmentLevel::Confirmed;
        let mut resub_timeout_ms = None;

        if let Some(config) = config {
            commitment = config.commitment;
            resub_timeout_ms = config.resub_timeout_ms;

            if let Some(timeout) = config.resub_timeout_ms {
                if timeout < 1000 {
                    log::info!(
                        "resub_timeout_ms should be at least 1000ms to avoid spamming resub"
                    );
                }
            }
        }

        Self {
            rpc_client,
            latest_slot: Arc::new(Mutex::new(0)),
            current_ts: Arc::new(Mutex::new(0)),
            _subscription_id: 0,
            _commitment: commitment,
            event_emitter: Arc::new(EventEmitter::new()),
            resub_timeout_ms,
            is_unsubscribing: Arc::new(Mutex::new(false)),
            receiving_data: Arc::new(Mutex::new(false)),
            subscription_task: None,
            shutdown_sender: None,
        }
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        let (shutdown_sender, mut shutdown_receiver) = tokio::sync::mpsc::channel::<()>(1);
        self.shutdown_sender = Some(Arc::new(Mutex::new(shutdown_sender)));

        let rpc_client = self.rpc_client.clone();
        let latest_slot = self.latest_slot.clone();
        let current_ts = self.current_ts.clone();
        let receiving_data = Arc::clone(&self.receiving_data);
        let is_unsubscribing = Arc::clone(&self.is_unsubscribing);
        let event_emitter = self.event_emitter.clone();

        let subscription_task = tokio::spawn(async move {
            let (mut stream, _shutdown_handle) = rpc_client
                .account_subscribe(&sysvar::clock::id(), None)
                .await
                .map_err(|e| SdkError::Generic(e.to_string()))
                .unwrap();

            loop {
                tokio::select! {
                    Some(res) = stream.next() => {
                         let mut latest_slot =  latest_slot.lock().await;
                         let is_unsubscribing = is_unsubscribing.lock().await;
                         let mut receiving_data = receiving_data.lock().await;
                         let mut current_ts = current_ts.lock().await;

                         if  *latest_slot< res.context.slot {
                             if !*is_unsubscribing {
                                 *receiving_data = true;
                             }
                         }

                        *latest_slot = res.context.slot;

                        let clock = deserialize_clock_data(&res.value.data).unwrap();
                        *current_ts = clock.unix_timestamp;
                        event_emitter.emit(
                            "clock_update",
                            Box::new(ClockSubscriberEvent::new(clock.unix_timestamp)),
                        );
                    },
                    _ = shutdown_receiver.recv() => {
                        break;
                    }
                }
            }
        });

        self.subscription_task = Some(Arc::new(Mutex::new(subscription_task)));

        if self.resub_timeout_ms.is_some() && !*self.is_unsubscribing.lock().await {
            self.set_timeout().await;
        }

        Ok(())
    }

    async fn set_timeout(&self) {
        if let Some(timeout_ms) = self.resub_timeout_ms {
            let duration = Duration::from_millis(timeout_ms);
            tokio::time::sleep(duration).await;
        }
    }

    pub async fn get_unix_ts(&self) -> i64 {
        *self.current_ts.lock().await
    }

    pub async fn unsubscribe(&mut self, on_resub: bool) -> SdkResult<()> {
        if !on_resub {
            self.resub_timeout_ms = None;
        }

        *self.is_unsubscribing.lock().await = true;

        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.lock().await.send(()).await;
        }

        if let Some(subscription_task) = self.subscription_task.take() {
            let _ = subscription_task.lock().await;
        }

        *self.is_unsubscribing.lock().await = false;

        Ok(())
    }
}

fn deserialize_clock_data(data: &UiAccountData) -> SdkResult<Clock> {
    // Assuming data is base64 encoded and the first part of the tuple contains the actual data
    match data {
        UiAccountData::Binary(base64_data, _) => {
            let bytes = STANDARD
                .decode(base64_data)
                .map_err(|e| SdkError::Generic(e.to_string()))?;

            let clock: Clock =
                bincode::deserialize(&bytes).map_err(|e| SdkError::Generic(e.to_string()))?;
            Ok(clock)
        }
        format => {
            return Err(SdkError::Generic(format!(
                "Unsupported data format: {format:?}"
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe_clock() {
        let rpc_client = PubsubClient::new("wss://api.devnet.solana.com")
            .await
            .expect("init PubsubClient");

        let mut clock_subscriber = ClockSubscriber::new(Arc::new(rpc_client), None);
        assert_eq!(clock_subscriber.get_unix_ts().await, 0);

        clock_subscriber.subscribe().await.expect("subscribe clock");
        assert_ne!(clock_subscriber.get_unix_ts().await, 0);
    }
}
