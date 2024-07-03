use std::{sync::Arc, time::Duration};

use async_utils::{retry_policy, spawn_retry_task};
use fnv::FnvHashMap;
use futures_util::{future::BoxFuture, FutureExt, StreamExt};
use log::{debug, warn};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::RpcAccountInfoConfig,
};
use solana_sdk::{
    account::Account,
    clock::Slot,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    hash::Hash,
    message::VersionedMessage,
    pubkey::Pubkey,
    signature::{keypair_from_seed, Keypair},
    signer::Signer,
    transaction::VersionedTransaction,
};
use tokio::{
    select,
    sync::{
        watch::{self, Receiver},
        RwLock,
    },
};
use types::*;
use websocket_account_subscriber::WebsocketAccountSubscriber;

pub mod accounts;
pub mod addresses;
pub mod async_utils;
pub mod blockhash_subscriber;
pub mod clock;
pub mod config;
pub mod constants;
pub mod dlob;
pub mod drift_client;
pub mod drift_client_config;
pub mod event_emitter;
pub mod events;
pub mod jupiter;
pub mod marketmap;
pub mod math;
pub mod memcmp;
pub mod oraclemap;
pub mod priority_fee;
pub mod slot_subscriber;
pub mod transaction_builder;
pub mod tx;
pub mod types;
pub mod user;
pub mod user_config;
pub mod user_stats;
pub mod user_stats_config;
pub mod usermap;
pub mod utils;
pub mod websocket_account_subscriber;
pub mod websocket_program_account_subscriber;

type AccountCache = Arc<RwLock<FnvHashMap<Pubkey, Receiver<(Account, Slot)>>>>;

/// Provides solana Account fetching API
pub trait AccountProvider: 'static + Sized + Send + Sync {
    // TODO: async fn when it stabilizes
    /// Return the Account information of `account`
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>>;
    /// the HTTP endpoint URL
    fn endpoint(&self) -> String;
    /// return configured commitment level of the provider
    fn commitment_config(&self) -> CommitmentConfig;
}

/// Account provider that always fetches from RPC
#[derive(Clone)]
pub struct RpcAccountProvider {
    client: Arc<RpcClient>,
}

impl RpcAccountProvider {
    pub fn new(endpoint: &str) -> Self {
        Self::with_commitment(endpoint, CommitmentConfig::confirmed())
    }
    /// Create a new RPC account provider with provided commitment level
    pub fn with_commitment(endpoint: &str, commitment: CommitmentConfig) -> Self {
        Self {
            client: Arc::new(RpcClient::new_with_commitment(
                endpoint.to_string(),
                commitment,
            )),
        }
    }
    async fn get_account_impl(&self, account: Pubkey) -> SdkResult<Account> {
        let account_data: Account = self.client.get_account(&account).await?;
        Ok(account_data)
    }
}

impl AccountProvider for RpcAccountProvider {
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>> {
        self.get_account_impl(account).boxed()
    }
    fn endpoint(&self) -> String {
        self.client.url()
    }
    fn commitment_config(&self) -> CommitmentConfig {
        self.client.commitment()
    }
}

/// Account provider using websocket subscriptions to receive and cache account updates
#[derive(Clone)]
pub struct WsAccountProvider {
    url: String,
    rpc_client: Arc<RpcClient>,
    /// map from account pubkey to (account data, last modified ts)
    account_cache: AccountCache,
}

struct AccountSubscription {
    account: Pubkey,
    url: String,
    rpc_client: Arc<RpcClient>,
    /// sink for account updates
    tx: Arc<watch::Sender<(Account, Slot)>>,
}

impl AccountSubscription {
    const RPC_CONFIG: RpcAccountInfoConfig = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64Zstd),
        data_slice: None,
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
        min_context_slot: None,
    };
    async fn stream_fn(self) {
        let ws_client =
            match PubsubClient::new(self.url.as_str().replace("http", "ws").as_str()).await {
                Ok(ws_client) => ws_client,
                Err(err) => {
                    warn!(target: "account", "connect client {:?} failed: {err:?}", self.account);
                    return;
                }
            };

        let result = ws_client
            .account_subscribe(&self.account, Some(Self::RPC_CONFIG))
            .await;

        if let Err(err) = result {
            warn!(target: "account", "subscribe account {:?} failed: {err:?}", self.account);
            return;
        }
        debug!(target: "account", "start account stream {:?}", self.account);
        let (mut account_stream, unsub_fn) = result.unwrap();

        let mut poll_interval = tokio::time::interval(Duration::from_secs(10));
        let _ = poll_interval.tick().await; // ignore, immediate first tick
        loop {
            select! {
                biased;
                response = account_stream.next() => {
                    if let Some(account_update) = response {
                        let slot = account_update.context.slot;
                        let account_data = account_update
                            .value
                            .decode::<Account>()
                            .expect("account");
                        self.tx.send_if_modified(|current| {
                            if slot > current.1 {
                                debug!(target: "account", "stream update writing to cache");
                                *current = (account_data, slot);
                                true
                            } else {
                                debug!(target: "account", "stream update old");
                               false
                            }
                        });
                    } else {
                        // websocket subscription/stream closed, try reconnect..
                        warn!(target: "account", "account stream closed: {:?}", self.account);
                        break;
                    }
                }
                _ = poll_interval.tick() => {
                    if let Ok(account_data) = self.rpc_client.get_account_with_config(&self.account, Default::default()).await {
                        self.tx.send_if_modified(|current| {
                            let slot = account_data.context.slot;
                            // only update with polled value if its newer
                            if slot > current.1 {
                                debug!(target: "account", "poll update, writing to cache");
                                *current = (account_data.value.unwrap(), slot);
                                true
                            } else {
                                debug!(target: "account", "poll update, too old");
                                false
                            }
                        });
                    } else {
                        // consecutive errors would indicate an issue, there's not much that can be done besides log/panic...
                    }
                }
            }
        }
        unsub_fn().await;
        warn!(target: "account", "stream ended: {:?}", self.account);
    }
}

impl WsAccountProvider {
    /// Create a new WsAccountProvider given an endpoint that serves both http(s) and ws(s)
    pub async fn new(url: &str) -> SdkResult<Self> {
        Self::new_with_commitment(url, CommitmentConfig::confirmed()).await
    }
    /// Create a new WsAccountProvider with provided commitment level
    pub async fn new_with_commitment(url: &str, commitment: CommitmentConfig) -> SdkResult<Self> {
        Ok(Self {
            url: url.to_string(),
            rpc_client: Arc::new(RpcClient::new_with_commitment(url.to_string(), commitment)),
            account_cache: Default::default(),
        })
    }
    /// Subscribe to account updates via web-socket and polling
    fn subscribe_account(&self, account: Pubkey, tx: watch::Sender<(Account, Slot)>) {
        let rpc_client = Arc::clone(&self.rpc_client);
        let tx = Arc::new(tx);
        let url = self.url.clone();
        spawn_retry_task(
            move || {
                let account_sub = AccountSubscription {
                    account,
                    url: url.clone(),
                    rpc_client: Arc::clone(&rpc_client),
                    tx: Arc::clone(&tx),
                };
                account_sub.stream_fn()
            },
            retry_policy::forever(5),
        );
    }
    /// Fetch an account and initiate subscription for future updates
    async fn get_account_impl(&self, account: Pubkey) -> SdkResult<Account> {
        {
            let cache = self.account_cache.read().await;
            if let Some(account_data_rx) = cache.get(&account) {
                let (account_data, _last_modified) = account_data_rx.borrow().clone();
                return Ok(account_data);
            }
        }

        // fetch initial account data, stream only updates on changes
        let account_data: Account = self.rpc_client.get_account(&account).await?;
        let (tx, rx) = watch::channel((account_data.clone(), 0));
        {
            let mut cache = self.account_cache.write().await;
            cache.insert(account, rx);
        }
        self.subscribe_account(account, tx);

        Ok(account_data)
    }
}

impl AccountProvider for WsAccountProvider {
    fn get_account(&self, account: Pubkey) -> BoxFuture<SdkResult<Account>> {
        self.get_account_impl(account).boxed()
    }
    fn endpoint(&self) -> String {
        self.rpc_client.url()
    }
    fn commitment_config(&self) -> CommitmentConfig {
        self.rpc_client.commitment()
    }
}

/// Drift wallet
#[derive(Clone, Debug)]
pub struct Wallet {
    /// The signing keypair, it could be authority or delegate
    pub signer: Arc<Keypair>,
    /// The drift 'authority' account
    /// user (sub)accounts are derived from this
    authority: Pubkey,
    /// The drift 'stats' account
    stats: Pubkey,
}

impl Wallet {
    /// Returns true if the wallet is configured for delegated signing
    pub fn is_delegated(&self) -> bool {
        self.authority != self.signer.pubkey() && self.signer.pubkey().is_on_curve()
    }
    /// Init wallet from a string that could be either a file path or the encoded key, uses default sub-account
    pub fn try_from_str(path_or_key: &str) -> SdkResult<Self> {
        let authority = utils::load_keypair_multi_format(path_or_key)?;
        Ok(Self::new(authority))
    }
    /// Construct a read-only wallet
    pub fn read_only(authority: Pubkey) -> Self {
        Self {
            signer: Arc::new(Keypair::from_bytes(&[0_u8; 64]).expect("empty signer")),
            authority,
            stats: Wallet::derive_stats_account(&authority, &constants::PROGRAM_ID),
        }
    }
    /// Init wallet from base58 encoded seed, uses default sub-account
    ///
    /// # panics
    /// if the key is invalid
    pub fn from_seed_bs58(seed: &str) -> Self {
        let authority: Keypair = Keypair::from_base58_string(seed);
        Self::new(authority)
    }
    /// Init wallet from seed bytes, uses default sub-account
    pub fn from_seed(seed: &[u8]) -> SdkResult<Self> {
        let authority: Keypair = keypair_from_seed(seed).map_err(|_| SdkError::InvalidSeed)?;
        Ok(Self::new(authority))
    }
    /// Init wallet with keypair
    ///
    /// `authority` keypair for tx signing
    pub fn new(authority: Keypair) -> Self {
        Self {
            stats: Wallet::derive_stats_account(&authority.pubkey(), &constants::PROGRAM_ID),
            authority: authority.pubkey(),
            signer: Arc::new(authority),
        }
    }
    /// Convert the wallet into a delegated one by providing the `authority` public key
    pub fn to_delegated(&mut self, authority: Pubkey) {
        self.stats = Wallet::derive_stats_account(&authority, &constants::PROGRAM_ID);
        self.authority = authority;
    }
    /// Calculate the address of a drift user account/sub-account
    pub fn derive_user_account(
        authority: &Pubkey,
        sub_account_id: u16,
        program: &Pubkey,
    ) -> Pubkey {
        let (account_drift_pda, _seed) = Pubkey::find_program_address(
            &[
                &b"user"[..],
                authority.as_ref(),
                &sub_account_id.to_le_bytes(),
            ],
            program,
        );
        account_drift_pda
    }

    /// Calculate the address of a drift stats account
    pub fn derive_stats_account(account: &Pubkey, program: &Pubkey) -> Pubkey {
        let (account_drift_pda, _seed) =
            Pubkey::find_program_address(&[&b"user_stats"[..], account.as_ref()], program);
        account_drift_pda
    }

    /// Signs the given tx `message` returning the tx on success
    pub fn sign_tx(
        &self,
        mut message: VersionedMessage,
        recent_block_hash: Hash,
        additional_signers: bool,
    ) -> SdkResult<VersionedTransaction> {
        message.set_recent_blockhash(recent_block_hash);
        let signer: &dyn Signer = self.signer.as_ref();

        let tx = if additional_signers {
            VersionedTransaction::try_new(message, &[signer, signer])
                .map_err(|e| SdkError::Signing(e))
        } else {
            VersionedTransaction::try_new(message, &[signer]).map_err(|e| SdkError::Signing(e))
        };

        tx
    }

    /// Return the wallet authority address
    pub fn authority(&self) -> &Pubkey {
        &self.authority
    }
    /// Return the wallet signing address
    pub fn signer(&self) -> Pubkey {
        self.signer.pubkey()
    }
    /// Return the drift user stats address
    pub fn stats(&self) -> &Pubkey {
        &self.stats
    }
    /// Return the address of the default sub-account (0)
    pub fn default_sub_account(&self) -> Pubkey {
        self.sub_account(0)
    }
    /// Calculate the drift user address given a `sub_account_id`
    pub fn sub_account(&self, sub_account_id: u16) -> Pubkey {
        Self::derive_user_account(self.authority(), sub_account_id, &constants::PROGRAM_ID)
    }
}

impl From<Keypair> for Wallet {
    fn from(value: Keypair) -> Self {
        Self::new(value)
    }
}
