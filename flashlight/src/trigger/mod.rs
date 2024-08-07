use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use drift::state::{perp_market::PerpMarket, spot_market::SpotMarket, user::MarketType};
use log::{error, info, warn};
use sdk::{
    dlob::{
        dlob_node::DLOBNode,
        dlob_subscriber::DLOBSubscriber,
        types::{DLOBSubscriptionConfig, DlobSource, SlotSource},
    },
    drift_client::DriftClient,
    slot_subscriber::SlotSubscriber,
    tx::priority_fee_calculator::PriorityFeeCalculator,
    types::{BaseTxParams, ProcessingTxParams, TxParams},
    usermap::UserMap,
    RpcAccountProvider,
};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::{config::BaseBotConfig, util::get_node_to_trigger_signature};

// time to wait between triggering an order
const TRIGGER_ORDER_COOLDOWN_MS: u64 = 10000;

#[allow(dead_code)]
pub struct TriggerBot {
    name: String,
    dry_run: bool,
    default_interval_ms: u64,

    drift_client: Arc<DriftClient<RpcAccountProvider>>,
    slot_subscriber: SlotSubscriber,
    dlob_subscriber: Option<DLOBSubscriber<RpcAccountProvider>>,
    triggering_nodes: HashMap<String, Instant>,
    periodic_task_mutex: Arc<Mutex<()>>,
    interval_tx: Option<oneshot::Sender<()>>,
    interval_handles: Option<JoinHandle<()>>,
    user_map: UserMap,

    priority_fee_calculator: PriorityFeeCalculator,
}

impl TriggerBot {
    pub fn new(
        drift_client: Arc<DriftClient<RpcAccountProvider>>,
        slot_subscriber: SlotSubscriber,
        user_map: UserMap,
        config: BaseBotConfig,
    ) -> Self {
        Self {
            name: config.bot_id,
            dry_run: config.dry_run,
            default_interval_ms: 1000,
            drift_client,
            slot_subscriber,
            dlob_subscriber: None,
            triggering_nodes: HashMap::new(),
            periodic_task_mutex: Arc::new(Mutex::new(())),
            interval_tx: None,
            interval_handles: None,
            user_map,
            priority_fee_calculator: PriorityFeeCalculator::new(Instant::now(), None),
        }
    }

    pub async fn init(&mut self) -> Result<(), String> {
        info!("{} initing", self.name);

        self.dlob_subscriber = Some(DLOBSubscriber::new(DLOBSubscriptionConfig {
            drift_client: self.drift_client.clone(),
            dlob_source: DlobSource::UserMap(self.user_map.clone()),
            update_frequency: Duration::from_millis(self.default_interval_ms - 500),
            slot_source: SlotSource::SlotSubscriber(self.slot_subscriber.clone()),
        }));
        if let Some(subscriber) = &self.dlob_subscriber {
            subscriber.subscribe().await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn reset(&mut self) -> Result<(), String> {
        if let Some(subscriber) = &mut self.dlob_subscriber {
            subscriber.unsubscribe().await;
        }

        self.user_map
            .unsubscribe()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn start_interval_loop(&mut self) {
        self.try_trigger().await;
    }

    async fn try_trigger(&mut self) {
        let _start = Instant::now();
        let _ran = false;

        match self.periodic_task_mutex.clone().try_lock() {
            Ok(_guard) => {
                let _perp_markets = self.drift_client.get_perp_market_accounts();
                let spot_markets = self.drift_client.get_spot_market_accounts();
                let user_map = self.user_map.clone();

                let drift_client = &self.drift_client;
                let triggering_nodes = Arc::new(Mutex::new(self.triggering_nodes.clone()));
                let priority_fee_calculator =
                    Arc::new(Mutex::new(self.priority_fee_calculator.clone()));

                if let Some(subscriber) = &self.dlob_subscriber {
                    let subscriber = Arc::new(subscriber.clone());
                    // let trigger_perp_markets: Vec<_> = perp_markets
                    //     .into_iter()
                    //     .map(|market| {
                    //         try_trigger_for_perp_market(
                    //             drift_client.clone(),
                    //             subscriber.clone(),
                    //             triggering_nodes.clone(),
                    //             user_map.clone(),
                    //             market,
                    //         )
                    //     })
                    //     .collect();

                    let trigger_spot_markets: Vec<_> = spot_markets
                        .into_iter()
                        .map(|market| {
                            try_trigger_trigger_fro_spot_market(
                                drift_client.clone(),
                                subscriber.clone(),
                                triggering_nodes.clone(),
                                user_map.clone(),
                                priority_fee_calculator.clone(),
                                market,
                            )
                        })
                        .collect();

                    // let all_futures = trigger_perp_markets
                    //     .into_iter()
                    //     .chain(trigger_spot_markets.into_iter())
                    //     .collect::<Vec<_>>();

                    // perp_market.iter().map(|m| )
                    let results = futures_util::future::join_all(trigger_spot_markets).await;
                    for result in results {
                        match result {
                            Ok(()) => log::info!("success triggering"),
                            Err(e) => log::error!("Fail to trigger: {e}"),
                        }
                    }
                }
            }
            Err(e) => {
                error!("Mutex is already locked: {e}")
            }
        }
    }
}

#[allow(dead_code)]
async fn try_trigger_for_perp_market(
    drift_client: Arc<DriftClient<RpcAccountProvider>>,
    subscriber: Arc<DLOBSubscriber<RpcAccountProvider>>,
    triggering_nodes: Arc<Mutex<HashMap<String, Instant>>>,
    user_map: UserMap,
    market: PerpMarket,
) -> Result<(), String> {
    let market_index = market.market_index;

    let oracle_price_data =
        drift_client.get_oracle_price_data_and_slot_for_perp_market(market_index);

    // if let Some(subscriber) = dlob_subscriber {
    let dlob = subscriber.get_dlob().await;
    let state = drift_client.get_state_account();
    let nodes_to_trigger = dlob.find_nodes_to_trigger(
        market_index,
        oracle_price_data.unwrap().data.price as u64,
        MarketType::Perp,
        state,
    );

    log::info!("Find nodes to trigger: {}", nodes_to_trigger.len());

    for node_to_trigger in nodes_to_trigger {
        let now = Instant::now();
        let node_to_fill_signature = get_node_to_trigger_signature(&node_to_trigger);
        if let Some(time_started_to_trigger_node) = triggering_nodes
            .lock()
            .unwrap()
            .get(&node_to_fill_signature)
        {
            if now - *time_started_to_trigger_node
                < Duration::from_millis(TRIGGER_ORDER_COOLDOWN_MS)
            {
                warn!("triggering node {node_to_fill_signature} too soon ({}ms since last trigger), skipping",(now - *time_started_to_trigger_node).as_millis());
                continue;
            }
        }

        // if node_to_trigger.

        triggering_nodes
            .lock()
            .unwrap()
            .insert(node_to_fill_signature, Instant::now());

        info!(
            "trying to trigger perp order on market {} (account {}) perp order {}",
            node_to_trigger.get_order().market_index,
            node_to_trigger.get_user_account(),
            node_to_trigger.get_order().order_id
        );

        let user = user_map
            .must_get(&node_to_trigger.get_user_account().to_string())
            .await
            .map_err(|e| e.to_string())?;

        let sub_account = drift_client.wallet().authority();
        let user_account = drift_client.get_user(Some(0)).ok_or("failed to get user")?;
        let msg = drift_client
            .init_tx(&sub_account, false)
            .map_err(|e| e.to_string())?
            .trigger_order_ix(
                &node_to_trigger.get_user_account(),
                &user,
                node_to_trigger.get_order(),
                Some(&user_account.pubkey),
                vec![],
            )
            .build();

        match drift_client.sign_and_send(msg, false).await {
            Ok(sig) => {
                info!(
                    "Triggered perp user (account: {}) perp order: {}",
                    node_to_trigger.get_user_account(),
                    node_to_trigger.get_order().order_id
                );
                info!("Tx: {sig}");
            }
            Err(e) => {
                // node_to_trigger.

                error!("Failed to trigger: {e}");
            }
        }
    }
    //  }

    Ok(())
}

async fn try_trigger_trigger_fro_spot_market(
    drift_client: Arc<DriftClient<RpcAccountProvider>>,
    subscriber: Arc<DLOBSubscriber<RpcAccountProvider>>,
    _triggering_nodes: Arc<Mutex<HashMap<String, Instant>>>,
    user_map: UserMap,
    priority_fee_calculator: Arc<Mutex<PriorityFeeCalculator>>,
    market: SpotMarket,
) -> Result<(), String> {
    let market_index = market.market_index;

    let oracle_price_data =
        drift_client.get_oracle_price_data_and_slot_for_spot_market(market_index);

    // if let Some(subscriber) = &subscriber {
    let dlob = subscriber.get_dlob().await;
    let state = drift_client.get_state_account();
    let nodes_to_trigger = dlob.find_nodes_to_trigger(
        market_index,
        oracle_price_data.unwrap().data.price as u64,
        MarketType::Spot,
        state,
    );

    log::info!("Find nodes to trigger: {}", nodes_to_trigger.len());

    for node_to_trigger in nodes_to_trigger {
        info!(
            "trying to trigger (account {}) spot order {}",
            node_to_trigger.get_user_account(),
            node_to_trigger.get_order().order_id
        );

        let user = user_map
            .must_get(&node_to_trigger.get_user_account().to_string())
            .await
            .map_err(|e| e.to_string())?;

        // TODO: modify tx_time_count
        let mut priority_fee_calculator = priority_fee_calculator.lock().unwrap();
        let user_priority_fee = priority_fee_calculator.update_priority_fee(Instant::now(), 0);

        let tx_params: Option<TxParams> = if user_priority_fee {
            let compute_units = 100_000;
            let compute_unit_price =
                priority_fee_calculator.calculate_compute_unit_price(compute_units, 1_000_000_000);
            Some(TxParams {
                base: BaseTxParams {
                    compute_units: Some(compute_units),
                    compute_units_price: Some(compute_unit_price),
                },
                processing: ProcessingTxParams::default(),
            })
        } else {
            None
        };

        match drift_client
            .trigger_order(
                &node_to_trigger.get_user_account(),
                user,
                node_to_trigger.get_order(),
                tx_params,
                None,
            )
            .await
        {
            Ok(sig) => {
                info!(
                    "Triggered user (account: {}) spot order: {}",
                    node_to_trigger.get_user_account(),
                    node_to_trigger.get_order().order_id
                );
                info!("Tx: {sig}");
            }
            Err(e) => {
                // node_to_trigger.

                error!("{e}");
            }
        }
    }
    // }
    Ok(())
}
