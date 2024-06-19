use solana_sdk::pubkey::Pubkey;

use crate::{drift_client::DriftClient, AccountProvider};

use super::{
    drift_priority_fee_method::{DriftMarketInfo, DriftPriorityFeeResponse},
    helius_priority_fee_method::HeliusPriorityFeeResponse,
    solana_priority_fee_method::SolanaPriorityFeeResponse,
};

pub(crate) enum PriorityFeeResponse<'a> {
    Solana(&'a [SolanaPriorityFeeResponse]),
    Helius(HeliusPriorityFeeResponse),
    Drift(DriftPriorityFeeResponse),
}

pub const DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS: u64 = 10_000;

pub trait PriorityFeeStrategy {
    fn calculate(&self, samples: PriorityFeeResponse) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PriorityFeeMethod {
    Solana,
    Helius,
    Drift,
}

impl From<&str> for PriorityFeeMethod {
    fn from(value: &str) -> Self {
        match value {
            "solana" => PriorityFeeMethod::Solana,
            "helius" => PriorityFeeMethod::Helius,
            "drift" => PriorityFeeMethod::Drift,
            val => panic!("Invalid string for PriorityFeeMethod: {val}"),
        }
    }
}

pub struct PriorityFeeSubscriberConfig<T: AccountProvider> {
    /// rpc connection, optional if using priorityFeeMethod.HELIUS
    //connection?: Connection;
    pub drift_client: Option<DriftClient<T>>,

    /// frequency to make RPC calls to update priority fee samples, in milliseconds
    pub frequency_ms: Option<u64>,

    /// addresses you plan to write lock, used to determine priority fees
    pub addresses: Option<Vec<Pubkey>>,

    /// drift market type and index, optionally provide at initialization time if using priorityFeeMethod.DRIFT
    pub drift_markets: Option<Vec<DriftMarketInfo>>,

    /// custom strategy to calculate priority fees, defaults to AVERAGE
    pub custom_strategy: Option<Box<dyn PriorityFeeStrategy>>,

    /// method for fetching priority fee samples
    pub priority_fee_method: Option<PriorityFeeMethod>,

    /// lookback window to determine priority fees, in slots.
    pub slots_to_check: Option<u8>,

    /// url for helius rpc, required if using priorityFeeMethod.HELIUS
    pub helius_rpc_url: Option<String>,

    /// url for drift cached priority fee endpoint, required if using priorityFeeMethod.DRIFT
    pub drift_priority_fee_endpoint: Option<String>,

    /// clamp any returned priority fee value to this value.
    pub max_fee_micro_lamports: Option<u64>,

    /// multiplier applied to priority fee before maxFeeMicroLamports, defaults to 1.0
    pub priority_fee_multiplier: Option<f64>,
}

pub struct PriorityFeeSubscriberMapConfig {
    /// frequency to make RPC calls to update priority fee samples, in milliseconds
    pub frequency_ms: Option<u64>,

    /// drift market type and associated market index to query
    pub drift_markets: Option<Vec<DriftMarketInfo>>,

    /// url for drift cached priority fee endpoint
    pub drift_priority_fee_endpoint: String,
}
