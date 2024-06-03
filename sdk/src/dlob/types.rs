use tokio::time::Duration;

use crate::{usermap::UserMap, AccountProvider, DriftClient};

use super::dlob::DLOB;

pub struct DLOBSubscriptionConfig<T: AccountProvider, U> {
    pub(crate) drift_client: DriftClient<T, U>,
    pub(crate) dlob_source: DlobSource,
    pub(crate) slot_source: SlotSource,
    pub(crate) update_frequency: Duration,
}

pub(crate) trait DLOBSubscriberEvents {
    fn update(dlob: DLOB);
    fn error();
}

pub enum DlobSource {
    UserMap(UserMap),
}

impl DlobSource {
    pub async fn get_dlob(&self, slot: u64) -> DLOB {
        match self {
            DlobSource::UserMap(usermap) => usermap.get_dlob(slot),
        }
    }
}

pub enum SlotSource {}

impl SlotSource {
    pub fn get_slot(&self) -> u64 {
        0
    }
}

// pub trait SlotSource {
//     fn get_slot(&self) -> u64;
// }
