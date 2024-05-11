use std::time::Duration;

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

    update_frequency: u64,

    interval_id: Option<Duration>,

    dlob: DLOB,

    event_emitter: EventEmitter,
}

impl<T, D, S> DLOBSubscriber<T, D, S>
where
    T: AccountProvider,
    D: DLOBSource + Send,
    S: SlotSource + Send,
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

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        if self.interval_id.is_none() {
            return Ok(());
        }

        self.update_dlob().await?;

        let update_frequency = self.update_frequency;
        tokio::task::spawn(async move {
            let mut timer =
                tokio::time::interval(tokio::time::Duration::from_millis(update_frequency));
            loop {
                {
                    self.update_dlob().await;
                    self.event_emitter.emit("update", Box::new(self.dlob.clone()));
                }
                let _ = timer.tick().await;
            }
        });

        Ok(())
    }

    async fn update_dlob(&mut self) -> SdkResult<()> {
        let slot = self.slot_source.get_slot();
        self.dlob = self.dlob_source.get_dlob(slot).await;

        Ok(())
    }
}
