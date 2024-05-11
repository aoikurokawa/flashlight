use crate::{AccountProvider, DriftClient};

use super::dlob::DLOB;

pub(crate) struct DLOBSubscriptionConfig<T: AccountProvider, D: DLOBSource, S: SlotSource> {
    pub(crate) drift_client: DriftClient<T>,
    pub(crate) dlob_source: D,
    pub(crate) slot_source: S,
    pub(crate) update_frequency: u64,
}

pub(crate) trait DLOBSubscriberEvents {
    fn update(dlob: DLOB);
    fn error();
}

pub(crate) trait DLOBSource {
    async fn get_dlob(&self, slot: u64) -> DLOB;
}

pub(crate) trait SlotSource {
    fn get_slot(&self) -> u64;
}
