use drift::state::user::User as UserAccount;

use crate::SdkResult;

pub struct DataAndSlot<T> {
    data: T,
    slot: u16,
}

enum UserAccountEvents {
    UserAccountUpdate { payload: UserAccount },
    Update,
    Error { e: String },
}

pub trait UserAccountSubscriber {
    async fn subscribe(&self, user_account: Option<UserAccount>) -> SdkResult<bool>;

    async fn fetch(&self) -> SdkResult<()>;

    async fn update_data(&self, user_account: UserAccount, slot: u16) -> SdkResult<()>;

    async fn unsubscribe(&self) -> SdkResult<()>;

    async fn get_user_account_and_slot(&self) -> SdkResult<DataAndSlot<UserAccount>>;
}
