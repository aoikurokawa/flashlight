use tokio::time::Duration;

use crate::{AccountProvider, DriftClient};

use super::dlob::DLOB;

pub(crate) struct DLOBSubscriptionConfig<T: AccountProvider, D: DLOBSource, S: SlotSource> {
    pub(crate) drift_client: DriftClient<T>,
    pub(crate) dlob_source: D,
    pub(crate) slot_source: S,
    pub(crate) update_frequency: Duration,
}

pub(crate) trait DLOBSubscriberEvents {
    fn update(dlob: DLOB);
    fn error();
}

pub(crate) trait DLOBSource {
    fn get_dlob(&self, slot: u64) -> impl std::future::Future<Output = DLOB> + Send;
}

pub(crate) trait SlotSource {
    fn get_slot(&self) -> u64;
}
