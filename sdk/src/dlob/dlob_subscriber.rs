use std::sync::Arc;

use drift::state::user::MarketType;
use tokio::{
    sync::Mutex,
    time::{self, Duration},
};

use crate::{
    event_emitter::EventEmitter,
    types::{SdkError, SdkResult},
    AccountProvider, DriftClient,
};

use super::{
    dlob::DLOB,
    order_book_levels::{L2OrderBook, L2OrderBookGenerator},
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
        if dlob_subscriber.clone().lock().await.interval_id.is_none() {
            return Ok(());
        }

        DLOBSubscriber::update_dlob(dlob_subscriber.clone()).await?;

        let update_frequency = dlob_subscriber.clone().lock().await.update_frequency;
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let subscriber = dlob_subscriber.clone();
        let update_task = tokio::spawn(async move {
            loop {
                time::sleep(update_frequency).await;
                match DLOBSubscriber::update_dlob(subscriber.clone()).await {
                    Ok(()) => tx.send(Ok(())).await.unwrap(),
                    Err(e) => tx.send(Err(e)).await.unwrap(),
                }
            }
        });

        let handle_events = tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(()) => dlob_subscriber.clone().lock().await.event_emitter.emit(
                        "update",
                        Box::new(dlob_subscriber.clone().lock().await.dlob.clone()),
                    ),
                    Err(e) => {
                        log::error!("Failed to subscribe to dlob: {e}");
                    }
                }
            }
        });

        let _ = tokio::try_join!(update_task, handle_events);

        Ok(())
    }

    async fn update_dlob(dlob_subscriber: Arc<Mutex<Self>>) -> SdkResult<()> {
        let mut subscriber = dlob_subscriber.lock().await;
        let slot = subscriber.slot_source.get_slot();
        subscriber.dlob = subscriber.dlob_source.get_dlob(slot).await;

        Ok(())
    }

    pub async fn get_dlob(&self) -> &DLOB {
        &self.dlob
    }

    pub async fn get_l2<L>(
        &self,
        market_name: Option<&str>,
        mut market_index: Option<u16>,
        mut market_type: Option<MarketType>,
        depth: u16,
        include_vamm: bool,
        num_vamm_orders: u16,
        fallback_l2_generators: Vec<L>,
    ) -> SdkResult<L2OrderBook>
    where
        L: L2OrderBookGenerator,
    {
        match market_name {
            Some(name) => {
                let derive_market_info = self.drift_client.market_lookup(name);

                match derive_market_info {
                    Some(info) => {
                        market_index = Some(info.index);
                        market_type = Some(info.kind);
                    }
                    None => return Err(SdkError::Generic(format!("Market ${name} not found"))),
                }
            }
            None => {
                if market_index.is_none() || market_type.is_none() {
                    return Err(SdkError::Generic(
                        "Either marketName or marketIndex and marketType must be provided"
                            .to_string(),
                    ));
                }
            }
        }

        let market_type = market_type.unwrap();
        let market_index = market_index.unwrap();
        let is_perp = market_type == MarketType::Perp;

        let oracle_data = if is_perp {
            let perp_market_account = self.drift_client.get_perp_market_info(market_index).await?;
            self.drift_client
                .get_oracle_price_data_and_slot_for_perp_market(perp_market_account.market_index)
        } else {
            self.drift_client
                .get_oracle_price_data_and_slot_for_spot_market(market_index)
        };

        if is_perp && include_vamm {
            if !fallback_l2_generators.is_empty() {
                return Err(SdkError::Generic(
                    "include_vamm can only be used if fallbackL2Generators is empty".to_string(),
                ));
            }

            
        }

        Ok(L2OrderBook {
            asks: todo!(),
            bids: todo!(),
            slot: todo!(),
        })
    }

    pub fn get_l3() {}
}
