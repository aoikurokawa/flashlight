use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use log::info;
use sdk::{
    dlob::{
        dlob_subscriber::DLOBSubscriber,
        types::{DLOBSubscriptionConfig, DlobSource, SlotSource},
    },
    slot_subscriber::SlotSubscriber,
    tx::priority_fee_calculator::PriorityFeeCalculator,
    usermap::UserMap,
    DriftClient, RpcAccountProvider,
};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::config::BaseBotConfig;

pub struct TriggerBot<U> {
    name: String,
    dry_run: bool,
    default_interval_ms: u64,

    drift_client: Arc<DriftClient<RpcAccountProvider, U>>,
    slot_subscriber: SlotSubscriber,
    dlob_subsciriber: Option<DLOBSubscriber<RpcAccountProvider, U>>,
    triggering_nodes: HashMap<String, u64>,
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
            dlob_subsciriber: None,
            triggering_nodes: HashMap::new(),
            interval_tx: None,
            interval_handles: None,
            user_map,
            priority_fee_calculator: PriorityFeeCalculator::new(Instant::now(), None),
        }
    }

    pub async fn init(&mut self) {
        info!("{} initing", self.name);

        self.dlob_subsciriber = Some(DLOBSubscriber::new(DLOBSubscriptionConfig {
            drift_client: self.drift_client.clone(),
            dlob_source: DlobSource::UserMap(self.user_map.clone()),
            update_frequency: Duration::from_millis(self.default_interval_ms - 500),
            slot_source: SlotSource::SlotSubscriber(self.slot_subscriber.clone()),
        }));
    }
}
