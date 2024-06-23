use std::sync::Arc;

use drift::state::user::MarketType;
use log::info;
use tokio::{
    sync::Mutex,
    time::{self, Duration},
};

use crate::{
    drift_client::DriftClient,
    event_emitter::EventEmitter,
    types::{SdkError, SdkResult},
    AccountProvider,
};

use super::{
    dlob::DLOB,
    order_book_levels::{
        L2OrderBook, L2OrderBookGenerator, L3OrderBook, VammL2Generator,
        DEFAULT_TOP_OF_BOOK_QUOTE_AMOUNTS,
    },
    types::{DLOBSubscriptionConfig, DlobSource, SlotSource},
};

struct DLOBSubscriberInner {
    dlob: DLOB,
}

// https://github.com/drift-labs/protocol-v2/blob/master/sdk/src/dlob/DLOBSubscriber.ts
#[derive(Clone)]
pub struct DLOBSubscriber<T: AccountProvider> {
    drift_client: Arc<DriftClient<T>>,

    dlob_source: DlobSource,

    slot_source: SlotSource,

    update_frequency: Duration,

    interval_id: Option<Duration>,

    dlob: Arc<Mutex<DLOBSubscriberInner>>,

    event_emitter: EventEmitter,
}

impl<T> DLOBSubscriber<T>
where
    T: AccountProvider + Clone,
{
    pub fn new(config: DLOBSubscriptionConfig<T>) -> Self {
        Self {
            drift_client: config.drift_client,
            dlob_source: config.dlob_source,
            slot_source: config.slot_source,
            update_frequency: config.update_frequency,
            interval_id: None,
            dlob: Arc::new(Mutex::new(DLOBSubscriberInner { dlob: DLOB::new() })),
            event_emitter: EventEmitter::new(),
        }
    }

    pub async fn subscribe(&self) -> SdkResult<()> {
        if self.interval_id.is_some() {
            return Ok(());
        }

        self.update_dlob().await?;

        let update_frequency = self.update_frequency;
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let subscriber = self.clone();
        tokio::spawn(async move {
            loop {
                time::sleep(update_frequency).await;
                match subscriber.update_dlob().await {
                    Ok(()) => tx.send(Ok(())).await.unwrap(),
                    Err(e) => tx.send(Err(e)).await.unwrap(),
                }
            }
        });

        let subscriber = self.clone();
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(()) => {
                        info!("updating dlob");
                        let dlob = subscriber.dlob.clone().lock().await.dlob.clone();
                        subscriber.event_emitter.emit("update", Box::new(dlob))
                    }
                    Err(e) => {
                        log::error!("Failed to subscribe to dlob: {e}");
                    }
                }
            }
        });

        // tokio::select! {
        //     res1 = update_task => if let Err(e) = res1 {
        //         log::error!("Update task failed: {:?}", e);
        //     },
        //     res2 = handle_events => if let Err(e) = res2 {
        //         log::error!("Event handling task failed: {:?}", e);
        //     }
        // }

        Ok(())
    }

    async fn update_dlob(&self) -> SdkResult<()> {
        let slot = self.slot_source.get_slot();
        let mut dlob = self.dlob.lock().await;
        let dlob_source = self.dlob_source.get_dlob(slot);

        info!("DLOB: {} {}", dlob_source.size().0, dlob_source.size().1);

        dlob.dlob = dlob_source;

        Ok(())
    }

    pub async fn get_dlob(&self) -> DLOB {
        self.dlob.lock().await.dlob.clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_l2(
        &mut self,
        market_name: Option<&str>,
        mut market_index: Option<u16>,
        mut market_type: Option<MarketType>,
        depth: usize,
        include_vamm: bool,
        num_vamm_orders: Option<usize>,
        mut fallback_l2_generators: Vec<Box<dyn L2OrderBookGenerator>>,
    ) -> SdkResult<L2OrderBook> {
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

        let oracle_price_data = if is_perp {
            let perp_market_account = self.drift_client.get_perp_market_info(market_index).await?;
            self.drift_client
                .get_oracle_price_data_and_slot_for_perp_market(perp_market_account.market_index)
                .ok_or_else(|| SdkError::Generic("".to_string()))?
        } else {
            self.drift_client
                .get_oracle_price_data_and_slot_for_spot_market(market_index)
                .ok_or_else(|| SdkError::Generic("".to_string()))?
        };

        if is_perp && include_vamm {
            if !fallback_l2_generators.is_empty() {
                return Err(SdkError::Generic(
                    "include_vamm can only be used if fallbackL2Generators is empty".to_string(),
                ));
            }

            let num_orders = match num_vamm_orders {
                Some(orders) => orders,
                None => depth,
            };
            let vamm_l2_generator = VammL2Generator::new(
                self.drift_client
                    .get_perp_market_account(market_index)
                    .ok_or(SdkError::Generic(
                        "could not find the perp market".to_string(),
                    ))?,
                &oracle_price_data.data,
                num_orders,
                None,
                Some(DEFAULT_TOP_OF_BOOK_QUOTE_AMOUNTS.to_vec()),
            )?;
            fallback_l2_generators = vec![Box::new(vamm_l2_generator)];
        }

        let mut dlob = self.dlob.lock().await.dlob.clone();
        Ok(dlob.get_l2::<VammL2Generator>(
            market_index,
            &market_type,
            self.slot_source.get_slot(),
            &oracle_price_data.data,
            depth,
            &mut fallback_l2_generators,
        ))
    }

    pub async fn get_l3(
        &mut self,
        market_name: Option<&str>,
        mut market_index: Option<u16>,
        mut market_type: Option<MarketType>,
    ) -> SdkResult<L3OrderBook> {
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

        let oracle_price_data = if is_perp {
            // let perp_market_account = self.drift_client.get_perp_market_info(market_index).await?;
            self.drift_client
                .get_oracle_price_data_and_slot_for_perp_market(market_index)
                .ok_or_else(|| SdkError::Generic("".to_string()))?
        } else {
            self.drift_client
                .get_oracle_price_data_and_slot_for_spot_market(market_index)
                .ok_or_else(|| SdkError::Generic("".to_string()))?
        };

        let mut dlob = self.dlob.lock().await.dlob.clone();
        Ok(dlob.get_l3(
            market_index,
            &market_type,
            self.slot_source.get_slot(),
            &oracle_price_data.data,
        ))
    }

    pub async fn unsubscribe(&mut self) {
        if self.interval_id.is_some() {
            self.interval_id = None
        }
    }
}
