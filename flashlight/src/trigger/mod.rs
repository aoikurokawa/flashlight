use sdk::{
    dlob::dlob_subscriber::DLOBSubscriber, slot_subscriber::SlotSubscriber, AccountProvider,
    DriftClient,
};
use tokio::{sync::oneshot, task::JoinHandle};

pub struct TriggerBot<T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    default_interval_ms: u64,

    drift_client: DriftClient<T, U>,
    slot_subscriber: SlotSubscriber,
    dlob_subsciriber: DLOBSubscriber<T, U>,
    interval_tx: Option<oneshot::Sender<()>>,
    interval_handles: Option<JoinHandle<()>>,
}
