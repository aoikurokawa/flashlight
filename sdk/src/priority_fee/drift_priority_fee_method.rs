use std::collections::HashMap;

use reqwest::StatusCode;
use serde::Deserialize;

use crate::types::{SdkError, SdkResult};

use super::helius_priority_fee_method::HeliusPriorityLevel;

#[derive(Debug, Clone)]
pub struct DriftMarketInfo {
    pub market_type: String,
    pub market_index: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriftPriorityFeeLevels {
    pub priority_fee_level: HashMap<HeliusPriorityLevel, u64>,
    pub market_type: String,
    pub market_index: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriftPriorityFeeResponse(pub Vec<DriftPriorityFeeLevels>);

pub(crate) async fn fetch_drift_priority_fee(
    url: &str,
    market_types: &[&str],
    market_indexes: &[u16],
) -> SdkResult<DriftPriorityFeeResponse> {
    let market_types: String = market_types.join(",");
    let market_indexes: String = market_indexes
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let url =
        format!("{url}/batchPriorityFees?marketType={market_types}&marketIndex={market_indexes}");
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    let status: StatusCode = response.status();
    let path: String = response.url().path().to_string();
    let body_text: String = response.text().await.unwrap_or_default();

    if status.is_success() {
        match serde_json::from_str::<DriftPriorityFeeResponse>(&body_text) {
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
