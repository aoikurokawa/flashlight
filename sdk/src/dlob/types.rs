use std::sync::Arc;

use tokio::time::Duration;

use crate::{
    drift_client::DriftClient, slot_subscriber::SlotSubscriber, usermap::UserMap, AccountProvider,
};

use super::dlob::DLOB;

pub struct DLOBSubscriptionConfig<T: AccountProvider, U> {
    pub drift_client: Arc<DriftClient<T, U>>,
    pub dlob_source: DlobSource,
    pub slot_source: SlotSource,
    pub update_frequency: Duration,
}

pub(crate) trait DLOBSubscriberEvents {
    fn update(dlob: DLOB);
    fn error();
}

#[derive(Clone)]
pub enum DlobSource {
    UserMap(UserMap),
}

impl DlobSource {
    pub fn get_dlob(&self, slot: u64) -> DLOB {
        match self {
            DlobSource::UserMap(usermap) => usermap.get_dlob(slot),
        }
    }
}

#[derive(Clone)]
pub enum SlotSource {
    SlotSubscriber(SlotSubscriber),
}

impl SlotSource {
    pub fn get_slot(&self) -> u64 {
        match self {
            SlotSource::SlotSubscriber(subscriber) => subscriber.get_slot(),
        }
    }
}

// pub trait SlotSource {
//     fn get_slot(&self) -> u64;
// }
