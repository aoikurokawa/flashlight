use std::collections::HashMap;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub trait AccountCallback: FnMut(Vec<u8>, u64) + Send + Sync {}

impl<T> AccountCallback for T where T: FnMut(Vec<u8>, u64) + Send + Sync {}

pub trait ErrorCallback: Fn(Arc<dyn std::error::Error + Send + Sync>) + Send + Sync {}

impl<T> ErrorCallback for T where T: Fn(Arc<dyn std::error::Error + Send + Sync>) + Send + Sync {}

#[derive(Clone)]
pub struct AccountToLoad {
    public_key: Pubkey,
    callbacks: HashMap<String, Arc<Mutex<dyn AccountCallback>>>,
}

pub struct BufferAndSlot {
    buffer: Vec<u8>,
    slot: u64,
}

pub struct BulkAccountLoader {
    client: Arc<RpcClient>,
    pub commitment: CommitmentConfig,
    polling_frequency: Duration,
    accounts_to_load: Arc<Mutex<HashMap<String, AccountToLoad>>>,
    buffer_and_slot_map: Arc<Mutex<HashMap<String, BufferAndSlot>>>,
    error_callbacks: Arc<Mutex<HashMap<String, Box<dyn ErrorCallback>>>>,
    interval_handle: Option<tokio::task::JoinHandle<()>>,
}

impl BulkAccountLoader {
    pub fn new(
        client: Arc<RpcClient>,
        commitment: CommitmentConfig,
        polling_frequency: Duration,
    ) -> Self {
        BulkAccountLoader {
            client,
            commitment,
            polling_frequency,
            accounts_to_load: Arc::new(Mutex::new(HashMap::new())),
            buffer_and_slot_map: Arc::new(Mutex::new(HashMap::new())),
            error_callbacks: Arc::new(Mutex::new(HashMap::new())),
            interval_handle: None,
        }
    }

    pub async fn add_account(
        &mut self,
        public_key: Pubkey,
        callback: Arc<Mutex<dyn AccountCallback>>,
    ) -> String {
        let callback_id = Uuid::new_v4().to_string();

        {
            let mut accounts_to_load = self.accounts_to_load.lock().await;

            if let Some(account_to_load) = accounts_to_load.get_mut(&public_key.to_string()) {
                account_to_load
                    .callbacks
                    .insert(callback_id.clone(), callback);
            } else {
                let mut callbacks = HashMap::new();
                callbacks.insert(callback_id.clone(), callback);
                let account_to_load = AccountToLoad {
                    public_key,
                    callbacks,
                };
                accounts_to_load.insert(public_key.to_string(), account_to_load);
            }
        }

        if self.accounts_to_load.lock().await.len() == 1 {
            self.start_polling().await;
        }

        callback_id
    }

    pub async fn remove_account(&mut self, public_key: Pubkey, callback_id: String) {
        {
            let mut accounts_to_load = self.accounts_to_load.lock().await;
            if let Some(account_to_load) = accounts_to_load.get_mut(&public_key.to_string()) {
                account_to_load.callbacks.remove(&callback_id);
                if account_to_load.callbacks.is_empty() {
                    accounts_to_load.remove(&public_key.to_string());
                    self.buffer_and_slot_map
                        .lock()
                        .await
                        .remove(&public_key.to_string());
                }
            }
        }

        if self.accounts_to_load.lock().await.is_empty() {
            self.stop_polling();
        }
    }

    pub async fn add_error_callback(&self, callback: Box<dyn ErrorCallback>) -> String {
        let mut error_callbacks = self.error_callbacks.lock().await;
        let callback_id = Uuid::new_v4().to_string();
        error_callbacks.insert(callback_id.clone(), Box::new(callback));
        callback_id
    }

    pub async fn remove_error_callback(&self, callback_id: String) {
        self.error_callbacks.lock().await.remove(&callback_id);
    }

    pub async fn load(&self) {
        let mut accounts_to_load = self.accounts_to_load.lock().await.clone();
        let mut account_chunks: Vec<&mut AccountToLoad> = accounts_to_load.values_mut().collect();
        let mut account_chunks: Vec<&mut [&mut AccountToLoad]> =
            account_chunks.chunks_mut(99).collect();

        let mut futures = Vec::new();

        for chunk in account_chunks.iter_mut() {
            futures.push(self.load_chunk(chunk));
        }

        futures_util::future::join_all(futures).await;
    }

    async fn load_chunk(&self, chunk: &mut [&mut AccountToLoad]) {
        let client = self.client.clone();
        let commitment = self.commitment;

        let pubkeys: Vec<Pubkey> = chunk.iter().map(|a| a.public_key).collect();
        let responses = match client
            .get_multiple_accounts_with_commitment(&pubkeys, commitment)
            .await
        {
            Ok(response) => response.value,
            Err(e) => {
                self.handle_error(Arc::new(e)).await;
                return;
            }
        };

        for (i, response) in responses.iter().enumerate() {
            if let Some(account_data) = response {
                let account_to_load = &mut chunk[i];
                let buffer = account_data.data.clone();
                let slot = account_data.lamports;

                let mut buffer_and_slot_map = self.buffer_and_slot_map.lock().await;
                let old_data = buffer_and_slot_map.get(&account_to_load.public_key.to_string());

                if old_data.is_none() || old_data.unwrap().slot < slot {
                    buffer_and_slot_map.insert(
                        account_to_load.public_key.to_string(),
                        BufferAndSlot {
                            buffer: buffer.clone(),
                            slot,
                        },
                    );
                    for callback in account_to_load.callbacks.values_mut() {
                        let mut callback = callback.lock().await;
                        callback(buffer.clone(), slot);
                    }
                }
            }
        }
    }

    async fn handle_error(&self, error: Arc<dyn std::error::Error + Send + Sync>) {
        let error_callbacks = self.error_callbacks.lock().await;
        for callback in error_callbacks.values() {
            callback(error.clone());
        }
    }

    async fn start_polling(&mut self) {
        let polling_frequency = self.polling_frequency;
        let loader = self.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(polling_frequency);
            loop {
                interval.tick().await;
                loader.load().await;
            }
        });

        self.interval_handle = Some(handle);
    }

    fn stop_polling(&mut self) {
        if let Some(handle) = &self.interval_handle {
            handle.abort();
            self.interval_handle = None;
        }
    }
}

impl Clone for BulkAccountLoader {
    fn clone(&self) -> Self {
        BulkAccountLoader {
            client: self.client.clone(),
            commitment: self.commitment,
            polling_frequency: self.polling_frequency,
            accounts_to_load: self.accounts_to_load.clone(),
            buffer_and_slot_map: self.buffer_and_slot_map.clone(),
            error_callbacks: self.error_callbacks.clone(),
            interval_handle: None,
        }
    }
}
