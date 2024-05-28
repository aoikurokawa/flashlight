use async_trait::async_trait;
use drift::state::user::User as UserAccount;

use crate::{
    types::{DataAndSlot, SdkError, UserStatsAccount},
    SdkResult,
};

#[async_trait]
pub trait AccountSubscriber<T> {
    async fn subscribe<F: FnMut(T) + std::marker::Send>(&mut self, on_change: F);
    async fn fetch(&mut self) -> SdkResult<()>;
    async fn unsubscribe(&self);
    fn set_data(&mut self, user_account: T, slot: Option<u64>);
}

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

pub struct ResubOpts {
    pub resub_timeout_ms: Option<u64>,
    pub log_resub_messages: Option<bool>,
}

pub trait UserStatsAccountEvents {
    fn user_stats_account_update(&self, payload: UserStatsAccount);
    fn update(&self);
    fn error(&self, e: SdkError);
}

#[async_trait]
pub trait UserStatsAccountSubscriber {
    async fn subscribe(&mut self, user_stats_account: Option<UserStatsAccount>) -> SdkResult<bool>;
    async fn fetch(&mut self) -> SdkResult<()>;
    async fn unsubscribe(&mut self);
    fn get_user_stats_account_and_slot(&self) -> SdkResult<Option<DataAndSlot<UserStatsAccount>>>;
}
