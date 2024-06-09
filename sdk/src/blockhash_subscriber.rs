use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use tokio::{
    sync::Mutex,
    time::{self, Duration},
};

use crate::SdkResult;

#[derive(Clone)]
pub struct BlockhashInner(Vec<Hash>);

#[derive(Clone)]
pub struct BlockhashSubscriber {
    is_subscribed: bool,

    latest_block_height: u64,

    latest_blockhash: Hash,

    last_twenty_hashes: Arc<Mutex<BlockhashInner>>,

    refresh_frequency: u64,

    rpc_client: Arc<RpcClient>,
}

impl BlockhashSubscriber {
    pub fn new(refresh_frequency: u64, endpoint: String) -> Self {
        BlockhashSubscriber {
            is_subscribed: false,
            latest_block_height: 0,
            latest_blockhash: Hash::default(),
            last_twenty_hashes: Arc::new(Mutex::new(BlockhashInner(Vec::with_capacity(20)))),
            refresh_frequency,
            rpc_client: Arc::new(RpcClient::new(endpoint)),
        }
    }

    pub async fn get_blockhash_size(&self) -> usize {
        let hashes = self.last_twenty_hashes.lock().await;
        hashes.0.len()
    }

    pub fn get_latest_block_height(&self) -> u64 {
        self.latest_block_height
    }

    pub fn get_latest_blockhash(&self) -> Hash {
        self.latest_blockhash
    }

    async fn update_blockhash(&mut self) -> SdkResult<()> {
        let blockhash = self.rpc_client.get_latest_blockhash().await?;
        let block_height = self.rpc_client.get_block_height().await?;

        self.latest_block_height = block_height;

        // avoid caching duplicate blockhashes
        let mut last_twenty_hashes = self.last_twenty_hashes.lock().await;
        if let Some(last_blockhash) = last_twenty_hashes.0.last() {
            if blockhash == *last_blockhash {
                return Ok(());
            }
        }

        last_twenty_hashes.0.push(blockhash);
        self.latest_blockhash = blockhash;

        Ok(())
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        if self.is_subscribed {
            return Ok(());
        }
        self.is_subscribed = true;

        self.update_blockhash().await?;

        let mut subscriber = self.clone();
        let update_frequency = Duration::from_millis(self.refresh_frequency);
        tokio::spawn(async move {
            loop {
                time::sleep(update_frequency).await;
                match subscriber.update_blockhash().await {
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

        Ok(())
    }

    pub async fn get_valid_blockhash(&self) -> Hash {
        let hashes = self.last_twenty_hashes.lock().await;
        *hashes.0.first().unwrap_or(&self.latest_blockhash)
    }
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
            let latest_blockhash = blockhash_subscriber.get_latest_blockhash();
            let valid_blockhash = blockhash_subscriber.get_valid_blockhash().await;
            // drop(blockhash_subscriber);
            dbg!(
                "{}: Latest blockhash: {:?}, Valid blockhash: {:?}",
                i,
                latest_blockhash,
                valid_blockhash
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}
