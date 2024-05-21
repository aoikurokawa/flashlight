use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::jupiter::field_as_string;

use super::{transaction_config::TransactionConfig, QuoteResponse};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    #[serde(with = "field_as_string")]
    pub user_public_key: Pubkey,

    pub quote_response: QuoteResponse,

    #[serde(flatten)]
    pub config: TransactionConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    #[serde(with = "base64_deserialize")]
    pub swap_transaction: Vec<u8>,
    pub last_valid_block_height: u64,
}

mod base64_deserialize {
    use super::*;
    use serde::{de, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let swap_transaction_string = String::deserialize(deserializer)?;
        base64::decode(swap_transaction_string)
            .map_err(|e| de::Error::custom(format!("base64 decoding error: {:?}", e)))
    }
}
