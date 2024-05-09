use drift::user::User as UserAccount;

enum UserAccountEvents {
    UserAccountUpdate {
        payload: UserAccount
    },
    Update,
    Error {
        e: Error,
    }
}

pub struct UserAccountSubscriber {
	event_emitter: StrictEventEmitter<EventEmitter, UserAccountEvents>,
	isSubscribed: boolean;

	subscribe(userAccount?: UserAccount): Promise<boolean>;
	fetch(): Promise<void>;
	updateData(userAccount: UserAccount, slot: number): void;
	unsubscribe(): Promise<void>;

	getUserAccountAndSlot(): DataAndSlot<UserAccount>;
}
