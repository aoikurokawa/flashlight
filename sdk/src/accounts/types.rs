use async_trait::async_trait;
use drift::state::user::User as UserAccount;

use crate::{
    types::{DataAndSlot, SdkError, UserStatsAccount},
    SdkResult,
};

enum UserAccountEvents {
    UserAccountUpdate { payload: UserAccount },
    Update,
    Error { e: String },
}

#[async_trait]
pub trait UserAccountSubscriber {
    async fn subscribe(&self, user_account: Option<UserAccount>) -> SdkResult<bool>;

    async fn fetch(&self) -> SdkResult<()>;

    async fn update_data(&self, user_account: UserAccount, slot: u16) -> SdkResult<()>;

    async fn unsubscribe(&self) -> SdkResult<()>;

    async fn get_user_account_and_slot(&self) -> SdkResult<DataAndSlot<UserAccount>>;
}

pub(crate) struct BufferAndSlot {
    pub(crate) slot: u64,
    pub(crate) buffer: Option<Vec<u8>>,
}

pub trait UserStatsAccountEvents {
    fn user_stats_account_update(&self, payload: UserStatsAccount);
    fn update(&self);
    fn error(&self, e: SdkError);
}

#[async_trait]
pub trait UserStatsAccountSubscriber {
    async fn subscribe(&mut self, user_stats_account: Option<UserStatsAccount>) -> bool;
    async fn fetch(&self);
    async fn unsubscribe(&mut self);
    fn get_user_account_and_slot(&self) -> DataAndSlot<UserStatsAccount>;
}
