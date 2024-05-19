use std::collections::HashMap;

use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use crate::types::SdkResult;

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
pub(crate) enum HeliusPriorityLevel {
    /// 25th percentile
    MIN,
    /// 25th percentile
    LOW,
    /// 50th percentile
    MEDIUM,
    /// 75th percentile
    HIGH,
    /// 95th percentile
    VERYHIGH,
    /// 100th percentile
    UNSAFEMAX,
}

impl From<&str> for HeliusPriorityLevel {
    fn from(value: &str) -> Self {
        match value {
            "min" => HeliusPriorityLevel::MIN,
            "low" => HeliusPriorityLevel::LOW,
            "medium" => HeliusPriorityLevel::MEDIUM,
            "high" => HeliusPriorityLevel::HIGH,
            "veryHigh" => HeliusPriorityLevel::VERYHIGH,
            "unsafeMax" => HeliusPriorityLevel::UNSAFEMAX,
            val => panic!("Invalid string for HeliusPriorityLevel: {val}"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct HeliusPriorityFeeLevels(HashMap<HeliusPriorityLevel, u64>);

#[derive(Debug, Deserialize)]
struct HeliusPriorityFeeResult {
    priority_fee_estimate: Option<u64>,
    priority_fee_levels: Option<HeliusPriorityFeeLevels>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HeliusPriorityFeeResponse {
    jsonrpc: String,
    result: HeliusPriorityFeeResult,
    id: String,
}

pub(crate) async fn fetch_helius_priority_fee(
    helius_rpc_url: &str,
    lookback_distance: u64,
    addresses: &[Pubkey],
) -> SdkResult<HeliusPriorityFeeResponse> {
    let addresses: String = addresses
        .iter()
        .map(|address| address.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let mut body = HashMap::new();
    body.insert("jsonrpc", "2.0".to_string());
    body.insert("id", "1".to_string());
    body.insert("method", "getPriorityFeeEstimate".to_string());
    body.insert(
        "params",
        format!(
            "[
{{
    accountKeys: [{addresses}],
    options: {{
        includeAllPriorityFeeLevels: true,
    }}
}}
]"
        ),
    );

    let client = reqwest::Client::new();
    let res = client.post(helius_rpc_url).json(&body).send().await?;

    eprintln!("Response: {res:?}");

    let json: HeliusPriorityFeeResponse = res.json().await?;

    Ok(json)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use solana_sdk::pubkey::Pubkey;

    use super::fetch_helius_priority_fee;

    #[tokio::test]
    async fn test_fetch_helius_priority_fee() {
        let rpc_url = "https://mainnet.helius-rpc.com/?api-key=ff28efe6-4fe6-4cf5-9525-01adeed6ee0b";
        let res = fetch_helius_priority_fee(
            rpc_url,
            1,
            &[Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap()],
        )
        .await;

        println!("{res:?}");
    }
}
