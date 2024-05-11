use std::sync::Arc;

use tokio::{
    sync::{Mutex, MutexGuard},
    time::{self, Duration},
};

use crate::{event_emitter::EventEmitter, types::SdkResult, AccountProvider, DriftClient};

use super::{
    dlob::DLOB,
    types::{DLOBSource, DLOBSubscriptionConfig, SlotSource},
};

// https://github.com/drift-labs/protocol-v2/blob/master/sdk/src/dlob/DLOBSubscriber.ts
pub struct DLOBSubscriber<T: AccountProvider, D: DLOBSource, S: SlotSource> {
    drift_client: DriftClient<T>,

    dlob_source: D,

    slot_source: S,

    update_frequency: Duration,

    interval_id: Option<Duration>,

    dlob: DLOB,

    event_emitter: EventEmitter,
}

impl<T, D, S> DLOBSubscriber<T, D, S>
where
    T: AccountProvider,
    D: DLOBSource + Send + Sync + 'static,
    S: SlotSource + Send + Sync + 'static,
{
    pub fn new(config: DLOBSubscriptionConfig<T, D, S>) -> Self {
        Self {
            drift_client: config.drift_client,
            dlob_source: config.dlob_source,
            slot_source: config.slot_source,
            update_frequency: config.update_frequency,
            interval_id: None,
            dlob: DLOB::new(),
            event_emitter: EventEmitter::new(),
        }
    }

    pub async fn subscribe(dlob_subscriber: Arc<Mutex<Self>>) -> SdkResult<()> {
        let mut dlob_subscriber = dlob_subscriber.clone().lock().await;
        if dlob_subscriber.interval_id.is_none() {
            return Ok(());
        }

        DLOBSubscriber::update_dlob(dlob_subscriber).await?;

        let update_frequency = dlob_subscriber.update_frequency;
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let update_task = tokio::spawn(async move {
            loop {
                time::sleep(update_frequency).await;
                match DLOBSubscriber::update_dlob(dlob_subscriber).await {
                    Ok(()) => tx.send(Ok(())).await.unwrap(),
                    Err(e) => tx.send(Err(e)).await.unwrap(),
                }
            }
        });

        let handle_events = tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(()) => dlob_subscriber
                        .event_emitter
                        .emit("update", Box::new(dlob_subscriber.dlob.clone())),
                    Err(e) => dlob_subscriber.event_emitter.emit("error", Box::new(e)),
                }
            }
        });

        let _ = tokio::try_join!(update_task, handle_events);

        Ok(())
    }

    async fn update_dlob(mut dlob_subscriber: MutexGuard<Self>) -> SdkResult<()> {
        // let mut dlob_subscriber = dlob_subscriber.clone().lock().unwrap();
        let slot = dlob_subscriber.slot_source.get_slot();
        dlob_subscriber.dlob = dlob_subscriber.dlob_source.get_dlob(slot).await;

        Ok(())
    }
}
