use std::collections::HashMap;

use sdk::{
    dlob::dlob_subscriber::DLOBSubscriber, slot_subscriber::SlotSubscriber, AccountProvider,
    DriftClient, usermap::UserMap,
};
use tokio::{sync::oneshot, task::JoinHandle};

pub struct TriggerBot<T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    default_interval_ms: u64,

    drift_client: DriftClient<T, U>,
    slot_subscriber: SlotSubscriber,
    dlob_subsciriber: Option<DLOBSubscriber<T, U>>,
    triggering_nodes: HashMap<String, u64>,
    interval_tx: Option<oneshot::Sender<()>>,
    interval_handles: Option<JoinHandle<()>>,
    user_map: UserMap,

    // priority_fee_calculator: PriorityFeeCa
}
