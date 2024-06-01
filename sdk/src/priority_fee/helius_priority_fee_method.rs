use std::collections::HashMap;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::types::{SdkError, SdkResult};

#[derive(Debug, Clone, Deserialize, Hash, PartialEq, Eq)]
pub enum HeliusPriorityLevel {
    /// 25th percentile
    Min,
    /// 25th percentile
    Low,
    /// 50th percentile
    Medium,
    /// 75th percentile
    High,
    /// 95th percentile
    VeryHigh,
    /// 100th percentile
    UnsafeMax,
}

impl From<&str> for HeliusPriorityLevel {
    fn from(value: &str) -> Self {
        match value {
            "min" => HeliusPriorityLevel::Min,
            "low" => HeliusPriorityLevel::Low,
            "medium" => HeliusPriorityLevel::Medium,
            "high" => HeliusPriorityLevel::High,
            "veryHigh" => HeliusPriorityLevel::VeryHigh,
            "unsafeMax" => HeliusPriorityLevel::UnsafeMax,
            val => panic!("Invalid string for HeliusPriorityLevel: {val}"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HeliusPriorityFeeLevels(pub(crate) HashMap<HeliusPriorityLevel, u64>);

#[derive(Debug, Deserialize)]
pub(crate) struct HeliusPriorityFeeResult {
    pub(crate) priority_fee_estimate: Option<u64>,
    pub(crate) priority_fee_levels: Option<HeliusPriorityFeeLevels>,
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
    pub(crate) result: HeliusPriorityFeeResult,
}

pub(crate) async fn fetch_helius_priority_fee(
    helius_rpc_url: &str,
    lookback_distance: u8,
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
                lookback_slots: Some(lookback_distance),
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
