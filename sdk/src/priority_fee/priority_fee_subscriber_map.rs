use std::collections::HashMap;

use tokio::{
    sync::mpsc,
    time::{self, Duration},
};

use crate::types::SdkResult;

use super::{
    drift_priority_fee_method::{
        fetch_drift_priority_fee, DriftMarketInfo, DriftPriorityFeeLevels, DriftPriorityFeeResponse,
    },
    types::{PriorityFeeSubscriberMapConfig, DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS},
};

#[derive(Debug, Clone)]
pub struct PriorityFeeSubscriberMap {
    frequency_ms: u64,
    // interval_id: Option<Interval>,
    drift_markets: Option<Vec<DriftMarketInfo>>,
    drift_priority_fee_endpoint: Option<String>,
    fees_map: HashMap<String, HashMap<u64, DriftPriorityFeeLevels>>,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl PriorityFeeSubscriberMap {
    pub fn new(config: PriorityFeeSubscriberMapConfig) -> Self {
        let frequency_ms = config
            .frequency_ms
            .unwrap_or(DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS);
        let mut fees_map = HashMap::new();
        fees_map.insert("perp".to_string(), HashMap::new());
        fees_map.insert("spot".to_string(), HashMap::new());

        Self {
            frequency_ms,
            // interval_id: None,
            drift_markets: config.drift_markets,
            drift_priority_fee_endpoint: Some(config.drift_priority_fee_endpoint),
            fees_map,
            stop_tx: None,
        }
    }

    pub fn update_fees_map(&mut self, drift_priority_fee_res: DriftPriorityFeeResponse) {
        drift_priority_fee_res.0.iter().for_each(|fee| {
            if let Some(fee_level) = self.fees_map.get_mut(&fee.market_type) {
                fee_level.insert(fee.market_index, fee.clone());
            }
        });
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        if self.stop_tx.is_some() {
            return Ok(());
        }

        self.load().await?;

        let (tx, mut rx) = mpsc::channel(1);
        self.stop_tx = Some(tx);
        let mut interval = time::interval(Duration::from_millis(self.frequency_ms));
        let mut self_clone = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(_e) = self_clone.load().await {
                        }
                    }
                    _ = rx.recv() => {
                        break;
                    }
                }
            }
        });

        // let this = subscriber.lock().await;

        // if this.interval_id.is_some() {
        //     return Ok(());
        // }

        // drop(this);
        // PriorityFeeSubscriberMap::load(subscriber.clone()).await?;

        // let mut this = subscriber.lock().await;

        // let interval = time::interval(Duration::from_millis(this.frequency_ms));
        // this.interval_id = Some(interval);

        // let self_clone = Arc::clone(&subscriber);

        // tokio::spawn(async move {
        //     let mut interval = self_clone.lock().await.interval_id.take().unwrap();
        //     loop {
        //         interval.tick().await;
        //         let _ = PriorityFeeSubscriberMap::load(self_clone.clone()).await;
        //     }
        // });

        Ok(())
    }

    pub async fn unsubscribe(&mut self) -> SdkResult<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(()).await;
        }

        Ok(())
    }

    async fn load(&mut self) -> SdkResult<()> {
        if let Some(drift_markets) = &self.drift_markets {
            let market_types: Vec<&str> = drift_markets
                .iter()
                .map(|m| m.market_type.as_str())
                .collect();
            let market_indices: Vec<u16> = drift_markets.iter().map(|m| m.market_index).collect();

            let fees = fetch_drift_priority_fee(
                &self.drift_priority_fee_endpoint.clone().unwrap(),
                &market_types,
                &market_indices,
            )
            .await?;
            self.update_fees_map(fees);
        }
        // let mut subscriber = subscriber.lock().await;
        // if let Some(drift_markets) = &subscriber.drift_markets {
        //     let endpoint = subscriber.drift_priority_fee_endpoint.clone().unwrap();
        //     let fees = fetch_drift_priority_fee(
        //         endpoint.as_str(),
        //         &drift_markets
        //             .iter()
        //             .map(|market| market.market_type.as_str())
        //             .collect::<Vec<&str>>(),
        //         &drift_markets
        //             .iter()
        //             .map(|market| market.market_index)
        //             .collect::<Vec<u16>>(),
        //     )
        //     .await?;

        //     let market_info = fees
        //         .0
        //         .iter()
        //         .map(|level| DriftMarketInfo {
        //             market_type: level.market_type.clone(),
        //             market_index: level.market_index as u16,
        //         })
        //         .collect();
        //     subscriber.update_market_type_and_index(market_info);
        // }

        Ok(())
    }

    pub fn update_market_type_and_index(&mut self, drift_markets: Vec<DriftMarketInfo>) {
        self.drift_markets = Some(drift_markets);
    }

    pub fn get_priority_fees(
        &self,
        market_type: &str,
        market_index: u64,
    ) -> Option<&DriftPriorityFeeLevels> {
        if let Some(level) = self.fees_map.get(market_type) {
            level.get(&market_index)
        } else {
            None
        }
    }
}
