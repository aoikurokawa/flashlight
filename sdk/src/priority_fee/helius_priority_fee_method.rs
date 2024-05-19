use std::collections::HashMap;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::types::{SdkError, SdkResult};

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

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HeliusPriorityFeeOptions {
    priority_level: Option<String>,
    include_all_priority_fee_levels: Option<bool>,
    transaction_encoding: Option<String>,
    lookback_slots: Option<u8>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HeliusPriorityFeeParams {
    #[serde(rename = "accountKeys")]
    account_keys: Option<Vec<String>>,
    options: Option<HeliusPriorityFeeOptions>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetPriorityFeeEstimateRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<HeliusPriorityFeeParams>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HeliusPriorityFeeResponse {
    jsonrpc: String,
    id: String,
    result: HeliusPriorityFeeResult,
}

pub(crate) async fn fetch_helius_priority_fee(
    helius_rpc_url: &str,
    lookback_distance: u64,
    addresses: &[Pubkey],
) -> SdkResult<HeliusPriorityFeeResponse> {
    let addresses = addresses
        .iter()
        .map(|address| address.to_string())
        .collect::<Vec<String>>();

    let request: GetPriorityFeeEstimateRequest = GetPriorityFeeEstimateRequest {
        jsonrpc: "2.0".to_string(),
        id: "1".to_string(),
        method: "getPriorityFeeEstimate".to_string(),
        params: vec![HeliusPriorityFeeParams {
            account_keys: Some(addresses),
            options: Some(HeliusPriorityFeeOptions {
                include_all_priority_fee_levels: Some(true),
                lookback_slots: None,
                priority_level: None,
                transaction_encoding: None,
            }),
        }],
    };

    let client = reqwest::Client::new();
    let response = client.post(helius_rpc_url).json(&request).send().await?;

    let status: StatusCode = response.status();
    let path: String = response.url().path().to_string();
    let body_text: String = response.text().await.unwrap_or_default();

    if status.is_success() {
        match serde_json::from_str::<HeliusPriorityFeeResponse>(&body_text) {
            Ok(json) => Ok(json),
            Err(e) => Err(SdkError::Generic(format!(
                "Deserialization Error: {e}, Raw JSON: {body_text}"
            ))),
        }
    } else {
        let body_json: serde_json::Result<serde_json::Value> = serde_json::from_str(&body_text);
        match body_json {
            Ok(body) => {
                let error_message: String = body["message"]
                    .as_str()
                    .unwrap_or("Unknown error")
                    .to_string();
                Err(SdkError::Generic(format!(
                    "Status: {status}, Path: {path}, Error Message: {error_message}"
                )))
            }
            Err(_) => Err(SdkError::Generic(format!(
                "Status: {status}, Path: {path}, Body Text: {body_text}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use solana_sdk::pubkey::Pubkey;

    use super::fetch_helius_priority_fee;

    #[tokio::test]
    async fn test_fetch_helius_priority_fee() {
        let rpc_url =
            "https://mainnet.helius-rpc.com/?api-key=ff28efe6-4fe6-4cf5-9525-01adeed6ee0b";
        let res = fetch_helius_priority_fee(
            rpc_url,
            1,
            &[Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap()],
        )
        .await;

        println!("{res:?}");
    }
}
