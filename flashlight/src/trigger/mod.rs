use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use drift::state::{perp_market::PerpMarket, user::MarketType};
use log::{info, warn};
use sdk::{
    dlob::{
        dlob_node::DLOBNode,
        dlob_subscriber::DLOBSubscriber,
        types::{DLOBSubscriptionConfig, DlobSource, SlotSource},
    },
    drift_client::DriftClient,
    slot_subscriber::SlotSubscriber,
    tx::priority_fee_calculator::PriorityFeeCalculator,
    usermap::UserMap,
    RpcAccountProvider,
};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::{config::BaseBotConfig, util::get_node_to_trigger_signature};

// time to wait between triggering an order
const TRIGGER_ORDER_COOLDOWN_MS: u64 = 10000;

pub struct TriggerBot<U> {
    name: String,
    dry_run: bool,
    default_interval_ms: u64,

    drift_client: Arc<DriftClient<RpcAccountProvider, U>>,
    slot_subscriber: SlotSubscriber,
    dlob_subscriber: Option<DLOBSubscriber<RpcAccountProvider, U>>,
    triggering_nodes: HashMap<String, Instant>,
    periodic_task_mutex: Arc<Mutex<()>>,
    interval_tx: Option<oneshot::Sender<()>>,
    interval_handles: Option<JoinHandle<()>>,
    user_map: UserMap,

    priority_fee_calculator: PriorityFeeCalculator,
}

impl<U> TriggerBot<U>
where
    U: Send + Sync + 'static + Clone,
{
    pub fn new(
        drift_client: Arc<DriftClient<RpcAccountProvider, U>>,
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

    async fn try_trigger_for_perp_market(&mut self, market: PerpMarket) -> Result<(), String> {
        let market_index = market.market_index;

        let oracle_price_data = self
            .drift_client
            .get_oracle_price_data_and_slot_for_perp_market(market_index);

        if let Some(subscriber) = &self.dlob_subscriber {
            let dlob = subscriber.get_dlob().await;
            let state = self.drift_client.get_state_account();
            let nodes_to_trigger = dlob.find_nodes_to_trigger(
                market_index,
                oracle_price_data.unwrap().data.price as u64,
                MarketType::Perp,
                state,
            );

            for node_to_trigger in nodes_to_trigger {
                let now = Instant::now();
                let node_to_fill_signature = get_node_to_trigger_signature(&node_to_trigger);
                if let Some(time_started_to_trigger_node) =
                    self.triggering_nodes.get(&node_to_fill_signature)
                {
                    if now - *time_started_to_trigger_node
                        < Duration::from_millis(TRIGGER_ORDER_COOLDOWN_MS)
                    {
                        warn!("triggering node {node_to_fill_signature} too soon ({}ms since last trigger), skipping",(now - *time_started_to_trigger_node).as_millis());
                        continue;
                    }
                }

                // if node_to_trigger.

                self.triggering_nodes
                    .insert(node_to_fill_signature, Instant::now());

                info!(
                    "trying to trigger perp order on market {} (account {}) perp order {}",
                    node_to_trigger.get_order().market_index,
                    node_to_trigger.get_user_account(),
                    node_to_trigger.get_order().order_id
                );

                let user = self
                    .user_map
                    .must_get(&node_to_trigger.get_user_account().to_string())
                    .await
                    .map_err(|e| e.to_string())?;

                // let mut ixs = Vec::new();
                // ixs.push(self.drift_client.get_tr)
            }
        }

        Ok(())
    }

    async fn try_trigger(&mut self) {
        let start = Instant::now();
        let mut ran = false;

        match self.periodic_task_mutex.try_lock() {
            Ok(_guard) => {
                let perp_market = self.drift_client.get_perp_market_accounts();

                // perp_market.iter().map(|m| )
                // futures_util::future::join_all(iter)
            }
            Err(e) => println!("Mutex is already locked"),
        }
    }
}
