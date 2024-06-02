use std::fs;

use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use crate::{config::DriftEnv, types::OracleSource};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpMarketConfig {
    pub full_name: Option<String>,
    pub category: Option<Vec<String>>,
    pub symbol: String,
    pub base_asset_symbol: String,
    pub market_index: u16,
    pub launch_ts: i64,
    #[serde(deserialize_with = "pubkey_from_str")]
    pub oracle: Pubkey,
    pub oracle_source: OracleSource,
}

fn pubkey_from_str<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

pub fn read_perp_markets(env: DriftEnv) -> Vec<PerpMarketConfig> {
    match env {
        DriftEnv::MainnetBeta => {
            let file_content =
                fs::read_to_string("../../constants/mainnet_perp_markets.json").expect("");
            serde_json::from_str(&file_content).expect("")
        }
        DriftEnv::Devnet => {
            let file_content =
                fs::read_to_string("./sdk/constants/dev_perp_markets.json").expect("");
            serde_json::from_str(&file_content).expect("")
        }
    }
}
