use std::{num::NonZeroUsize, time::Duration};

use lru::LruCache;
use sdk::slot_subscriber::SlotSubscriber;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::VersionedTransaction,
};

use crate::types::JitoStrategy;

#[allow(dead_code)]
struct TipStream {
    time: String,
    ts: u16,
    landed_tips_25th_percentile: u16,     // in SOL
    landed_tips_50th_percentile: u16,     // in SOL
    landed_tips_75th_percentile: u16,     // in SOL
    landed_tips_95th_percentile: u16,     // in SOL
    landed_tips_99th_percentile: u16,     // in SOL
    ema_landed_tips_50th_percentile: u16, // in SOL
}

#[allow(dead_code)]
enum DropReason {
    Pruned,
    BlockhashExpired,
    BlockhashNotFound,
}

#[allow(dead_code)]
struct JitoLeader {
    current_slot: u64,
    next_leader_slot: u64,
    next_leader_identity: String,
}

#[allow(dead_code)]
struct Bundle {
    tx: String,
    ts: u16,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct BundleStats {
    accepted: u16,
    state_auction_bid_rejected: u16,
    winning_batch_bid_rejected: u16,
    simulation_failure: u16,
    internal_error: u16,
    dropped_bundle: u16,

    /// extra stats
    dropped_pruned: u16,
    dropped_blockhash_expired: u16,
    dropped_blockhash_bot_found: u16,
}

#[allow(dead_code)]
pub struct BundleSender {
    // ws: Option<WebSocket>,
    // searcher_client: SearcherClient,
    leader_schedule_interval_id: Option<Duration>,
    check_sent_txs_interval_id: Option<Duration>,
    is_subscribed: bool,
    shutting_down: bool,
    jito_tip_accounts: Vec<Pubkey>,
    next_jito_leader: Option<JitoLeader>,
    updating_jito_schedule: bool,
    checking_sent_txs: bool,

    /// if there is a big difference, probably jito ws connection is bad, should resub
    bundles_sent: u16,

    bundle_results_received: u16,

    /// `bundleIdToTx` will be populated immediately after sending a bundle.
    bundle_id_to_tx: LruCache<String, Bundle>,

    /// `sent_tx_cache` will only be populated after a bundle result is received.
    /// reason being that sometimes results come really late (like minutes after sending)
    /// unsure if this is a jito issue or this bot is inefficient and holding onto things
    /// for that long. Check txs from this map to see if they landed.
    sent_tx_cache: LruCache<String, u16>,

    /// -1 for each accepted bundle, +1 for each rejected (due to bid, don't count sim errors).
    fail_bundle_count: u16,

    count_landed_bundles: u16,

    count_dropped_bundles: u16,

    last_tip_stream: Option<TipStream>,

    bundle_stats: BundleStats,

    // connection: Connection,
    tip_payer_keypair: Keypair,
    slot_subscriber: SlotSubscriber,

    /// tip algo params
    pub(crate) strategy: JitoStrategy,

    // cant be lower than this
    min_bundle_tip: u16,

    max_bundle_tip: u64,

    max_fail_bundle_count: u16,
    // bigger == more superlinear, delay the ramp up to prevent overpaying too soon
    tip_multiplier: u16,
}

impl BundleSender {
    pub fn new(tip_payer_keypair: Keypair, slot_subscriber: SlotSubscriber) -> Self {
        Self {
            leader_schedule_interval_id: None,
            check_sent_txs_interval_id: None,
            is_subscribed: false,
            shutting_down: false,
            jito_tip_accounts: Vec::new(),
            next_jito_leader: None,
            updating_jito_schedule: false,
            checking_sent_txs: false,
            bundles_sent: 0,
            bundle_results_received: 0,
            bundle_id_to_tx: LruCache::new(NonZeroUsize::new(500).unwrap()),
            sent_tx_cache: LruCache::new(NonZeroUsize::new(500).unwrap()),
            fail_bundle_count: 0,
            count_landed_bundles: 0,
            count_dropped_bundles: 0,
            last_tip_stream: None,
            bundle_stats: BundleStats::default(),
            tip_payer_keypair,
            slot_subscriber,
            strategy: JitoStrategy::JitoOnly,
            min_bundle_tip: 10_000,
            max_bundle_tip: 100_000,
            max_fail_bundle_count: 100,
            tip_multiplier: 3,
        }
    }

    pub fn slots_until_next_leader(&self) -> Option<u64> {
        match &self.next_jito_leader {
            Some(leader) => Some(leader.next_leader_slot - self.slot_subscriber.current_slot()),
            None => None,
        }
    }

    /// Alternatively, don't create the bundle now, but batch them and send them together with 1
    /// tip.
    pub(crate) async fn send_transaction(
        &self,
        _signed_tx: &VersionedTransaction,
        _metadata: Option<String>,
        _tx_sig: Option<Signature>,
    ) {
        if !self.is_subscribed {
            log::warn!("You should call bundle_sender.subscribe() before send_transaction()");
        }

        todo!()
    }
}
