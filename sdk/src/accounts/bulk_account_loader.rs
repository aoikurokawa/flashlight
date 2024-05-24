// use solana_rpc::rpc::rpc_accounts::gen_client::Client;
//

use std::collections::HashMap;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use uuid::Uuid;

use crate::types::SdkResult;

use super::BufferAndSlot;

struct AccountToLoad<F> {
    public_key: Pubkey,
    callbacks: HashMap<String, F>,
}

const GET_MULTIPLE_ACCOUNTS_CHUNK_SIZE: u8 = 99;

pub struct BulkAccountLoader<Account, Load> {
    rpc_client: RpcClient,
    polling_frequency: u64,
    accounts_to_load: HashMap<Pubkey, AccountToLoad<F>>,
    buffer_and_slot_map: HashMap<String, BufferAndSlot>,
    load_promise: Load,
}

impl<Account, Load> BulkAccountLoader<Account, Load> {
    pub fn new(rpc_client: RpcClient, polling_frequency: u64) -> Self {
        Self {
            rpc_client,
            polling_frequency,
            accounts_to_load: HashMap::new(),
            buffer_and_slot_map: HashMap::new(),
        }
    }

    pub async fn add_account(&mut self, public_key: &Pubkey, callback: F) -> SdkResult<()> {
        // let existing_size = self.accounts_to_load.len();

        let callback_id = Uuid::new_v4();
        match self.accounts_to_load.get_mut(public_key) {
            Some(account_to_load) => {
                account_to_load
                    .callbacks
                    .insert(callback_id.to_string(), callback);
            }
            None => {
                let mut callbacks = HashMap::new();
                callbacks.insert(callback_id.to_string(), callback);
                let new_account_to_load = AccountToLoad {
                    public_key,
                    callbacks,
                };
                self.accounts_to_load
                    .insert(public_key, new_account_to_load);
            }
        }

        if self.accounts_to_load.is_empty() {
            self.start_polling();
        }

        Ok(())
    }

    pub fn start_polling(&self) {}
}
