use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use crate::types::OracleSource;

#[derive(Debug, Clone, Deserialize)]
struct PerpMarketConfig {
    full_name: Option<String>,
    category: Option<Vec<String>>,
    symbol: String,
    base_asset_symbol: String,
    market_index: i32,
    launch_ts: i64,
    #[serde(deserialize_with = "pubkey_from_str")]
    oracle: Pubkey,
    oracle_source: OracleSource,
}

fn pubkey_from_str<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
