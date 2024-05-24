// use solana_rpc::rpc::rpc_accounts::gen_client::Client;
//

use std::{
    collections::HashMap,
    future::Future,
    ops::Sub,
    time::{Duration, Instant, SystemTime},
};

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

pub struct BulkAccountLoader<F, Fut, G> {
    rpc_client: RpcClient,
    polling_frequency: u64,
    accounts_to_load: HashMap<Pubkey, AccountToLoad<F>>,
    buffer_and_slot_map: HashMap<Pubkey, BufferAndSlot>,
    error_callbacks: HashMap<String, G>,
    load_promise: Option<Fut>,
    last_time_loading_promise_cleared: Instant,
}

impl<F, Fut, G> BulkAccountLoader<F, Fut, G>
where
    Fut: Future<Output = ()>,
{
    pub fn new(rpc_client: RpcClient, polling_frequency: u64) -> Self {
        Self {
            rpc_client,
            polling_frequency,
            accounts_to_load: HashMap::new(),
            buffer_and_slot_map: HashMap::new(),
            error_callbacks: HashMap::new(),
            load_promise: None,
            last_time_loading_promise_cleared: Instant::now(),
        }
    }

    pub async fn add_account(mut self, public_key: &Pubkey, callback: F) -> SdkResult<String> {
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
                    public_key: *public_key,
                    callbacks,
                };
                self.accounts_to_load
                    .insert(*public_key, new_account_to_load);
            }
        }

        if self.accounts_to_load.is_empty() {
            self.start_polling();
        }

        // resolve the current load_promise in case client wants to call load
        let load_promise = self.load_promise.unwrap();
        load_promise.await;

        Ok(callback_id.to_string())
    }

    pub fn remove_account(&mut self, public_key: &Pubkey, callback_id: String) -> SdkResult<()> {
        let mut is_empty = false;
        let mut pubkey = Pubkey::new_unique();

        if let Some(existing_account_to_load) = self.accounts_to_load.get_mut(public_key) {
            existing_account_to_load.callbacks.remove(&callback_id);

            if existing_account_to_load.callbacks.is_empty() {
                self.buffer_and_slot_map.remove(public_key);
                is_empty = true;
                pubkey = existing_account_to_load.public_key;
            }
        }

        if is_empty {
            self.accounts_to_load.remove(&pubkey);
        }

        if self.accounts_to_load.is_empty() {
            self.stop_polling();
        }

        Ok(())
    }

    pub fn add_error_callbacks(&mut self, callback: G) -> SdkResult<String> {
        let callback_id = Uuid::new_v4();
        self.error_callbacks
            .insert(callback_id.to_string(), callback);

        Ok(callback_id.to_string())
    }

    pub fn remove_error_callbacks(&mut self, callback_id: String) -> SdkResult<()> {
        self.error_callbacks.remove(&callback_id);

        Ok(())
    }

    fn chunks<T: Clone>(array: &[T], size: usize) -> Vec<Vec<T>> {
        let mut result = Vec::new();
        let mut index = 0;

        while index < array.len() {
            let end = std::cmp::min(index + size, array.len());
            result.push(array[index..end].to_vec());
            index += size;
        }

        result
    }

    pub async fn load(&mut self) -> SdkResult<()> {
        if let Some(_load_promise) = &self.load_promise {
            let now = Instant::now();

            if now.sub(self.last_time_loading_promise_cleared) > Duration::new(60, 0) {
                self.load_promise = None;
            } else {
                // return self.load_promise;
            }
        }

        // self.load_promise = 

        Ok(())
    }

    pub fn start_polling(&self) {}

    pub fn stop_polling(&self) {}
}
