use std::{collections::VecDeque, sync::Arc};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use tokio::{
    sync::Mutex,
    time::{self, Duration},
};

use crate::SdkResult;

#[derive(Clone)]
pub struct BlockhashState {
    latest_block_height: u64,

    latest_blockhash: Hash,

    last_twenty_hashes: VecDeque<Hash>,
}

#[derive(Clone)]
pub struct BlockhashSubscriber {
    is_subscribed: bool,

    state: Arc<Mutex<BlockhashState>>,

    refresh_frequency: u64,

    rpc_client: Arc<RpcClient>,
}

impl BlockhashSubscriber {
    pub fn new(refresh_frequency: u64, endpoint: String) -> Self {
        BlockhashSubscriber {
            is_subscribed: false,
            state: Arc::new(Mutex::new(BlockhashState {
                latest_block_height: 0,
                latest_blockhash: Hash::default(),
                last_twenty_hashes: VecDeque::with_capacity(20),
            })),
            refresh_frequency,
            rpc_client: Arc::new(RpcClient::new(endpoint)),
        }
    }

    pub async fn get_blockhash_size(&self) -> usize {
        let state = self.state.lock().await;
        state.last_twenty_hashes.len()
    }

    pub async fn get_latest_block_height(&self) -> u64 {
        let state = self.state.lock().await;
        state.latest_block_height
    }

    pub async fn get_latest_blockhash(&self) -> Hash {
        let state = self.state.lock().await;
        state.latest_blockhash
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        if self.is_subscribed {
            return Ok(());
        }
        self.is_subscribed = true;

        // update_blockhash(&self.rpc_client, &self.state).await?;

        let state = self.state.clone();
        let rpc_client = self.rpc_client.clone();
        let update_frequency = Duration::from_millis(self.refresh_frequency);
        tokio::spawn(async move {
            loop {
                time::sleep(update_frequency).await;
                match update_blockhash(&rpc_client, &state).await {
                    Ok(()) => log::info!("success updating"),
                    Err(e) => log::error!("cannot update: {e}"),
                }
            }
        });
        // let blockhash_subscriber = blockhash_subscriber.clone();
        // let blockhash_subscriber_reader = blockhash_subscriber.read().await;
        // let refresh_frequency = blockhash_subscriber_reader.refresh_frequency;
        // drop(blockhash_subscriber_reader);

        // tokio::spawn(async move {
        //     loop {
        //         let mut blockhash_subscriber_writer = blockhash_subscriber.write().await;
        //         let blockhash = blockhash_subscriber_writer
        //             .rpc_client
        //             .get_latest_blockhash()
        //             .await
        //             .expect("blockhash");
        //         blockhash_subscriber_writer
        //             .last_twenty_hashes
        //             .push(blockhash);
        //         blockhash_subscriber_writer.latest_blockhash = blockhash;
        //         if blockhash_subscriber_writer.last_twenty_hashes.len() > 20 {
        //             blockhash_subscriber_writer.last_twenty_hashes.remove(0);
        //         }
        //         drop(blockhash_subscriber_writer);
        //         tokio::time::sleep(tokio::time::Duration::from_secs(refresh_frequency)).await;
        //     }
        // });

        log::info!("Done subscribing state");

        Ok(())
    }

    pub async fn get_valid_blockhash(&self) -> Hash {
        let state = self.state.lock().await;
        *state
            .last_twenty_hashes
            .front()
            .unwrap_or(&state.latest_blockhash)
    }
}

async fn update_blockhash(
    rpc_client: &Arc<RpcClient>,
    state: &Arc<Mutex<BlockhashState>>,
) -> SdkResult<()> {
    let blockhash = rpc_client.get_latest_blockhash().await?;
    let block_height = rpc_client.get_block_height().await?;

    let mut state = state.lock().await;
    state.latest_block_height = block_height;

    // avoid caching duplicate blockhashes
    if let Some(last_blockhash) = state.last_twenty_hashes.back() {
        if blockhash == *last_blockhash {
            return Ok(());
        }
    }

    state.last_twenty_hashes.push_back(blockhash);

    if state.last_twenty_hashes.len() > 20 {
        state.last_twenty_hashes.pop_front();
    }

    state.latest_blockhash = blockhash;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockhash_subscribe() {
        let rpc = "https://api.devnet.solana.com";
        let mut blockhash_subscriber = BlockhashSubscriber::new(2, rpc.to_string());
        blockhash_subscriber
            .subscribe()
            .await
            .expect("subscribe blockhash");

        let blockhash_subscriber = Arc::new(blockhash_subscriber);

        for i in 0..=10 {
            let latest_blockhash = blockhash_subscriber.get_latest_blockhash().await;
            let valid_blockhash = blockhash_subscriber.get_valid_blockhash().await;
            // drop(blockhash_subscriber);
            println!(
                "{}: Latest blockhash: {:?}, Valid blockhash: {:?}",
                i, latest_blockhash, valid_blockhash
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}
