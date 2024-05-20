use std::{collections::HashMap, str::FromStr};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{address_lookup_table_account::AddressLookupTableAccount, pubkey::Pubkey};

use crate::{
    jupiter::serde_helpers::field_as_string,
    types::{SdkError, SdkResult},
    AccountProvider, DriftClient,
};

mod serde_helpers;

#[derive(Serialize, Deserialize, Default, PartialEq, Clone, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

impl FromStr for SwapMode {
    type Err = SdkError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ExactIn" => Ok(Self::ExactIn),
            "ExactOut" => Ok(Self::ExactOut),
            _ => Err(SdkError::Generic(format!("{} is not a valid SwapMode", s))),
        }
    }
}

pub struct MarketInfo {
    id: String,
    in_amount: u64,
    input_mint: Pubkey,
    label: String,
    lp_fee: Fee,
    not_enough_liquidity: bool,
    out_amount: u64,
    output_mint: Pubkey,
    platform_fee: Fee,
    price_impact_pct: String,
}

pub struct Fee {
    amount: u64,
    mint: Pubkey,
    pct: String,
}

pub struct Route {
    amount: u64,
    in_amount: u64,
    market_infos: Vec<MarketInfo>,
    other_amount_threshold: u64,
    out_amount: u64,
    price_impact_pct: String,
    slippage_bps: u64,
    swap_mode: SwapMode,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    #[serde(with = "field_as_string")]
    pub amm_key: Pubkey,
    pub label: String,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    /// An estimation of the input amount into the AMM
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    /// An estimation of the output amount into the AMM
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_mint: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFee {
    #[serde(with = "field_as_string")]
    pub amount: u64,
    pub fee_bps: u8,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub amount: u64,
    pub swap_mode: Option<SwapMode>,
    /// Allowed slippage in basis points
    pub slippage_bps: u16,
    /// Platform fee in basis points
    pub platform_fee_bps: Option<u8>,
    pub dexes: Option<Vec<String>>,
    pub excluded_dexes: Option<Vec<String>>,
    /// Quote only direct routes
    pub only_direct_routes: Option<bool>,
    /// Quote fit into legacy transaction
    pub as_legacy_transaction: Option<bool>,
    /// Find a route given a maximum number of accounts involved,
    /// this might dangerously limit routing ending up giving a bad price.
    /// The max is an estimation and not the exact count
    pub max_accounts: Option<usize>,
    // Quote type to be used for routing, switches the algorithm
    pub quote_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    /// Not used by build transaction
    #[serde(with = "field_as_string")]
    pub other_amount_threshold: u64,
    pub swap_mode: SwapMode,
    pub slippage_bps: u16,
    pub platform_fee: Option<PlatformFee>,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(default)]
    pub context_slot: u64,
    #[serde(default)]
    pub time_taken: f64,
}

pub struct JupiterClient<T: AccountProvider> {
    url: String,
    rpc_client: RpcClient,
    lookup_table_cache: HashMap<String, AddressLookupTableAccount>,
}

impl<T: AccountProvider> JupiterClient<T> {
    pub fn new(drift_client: DriftClient<T>, url: Option<String>) -> Self {
        let url = match url {
            Some(url) => url,
            None => "https://quote-api.jup.ag".to_string(),
        };

        Self {
            url,
            drift_client,
            lookup_table_cache: HashMap::new(),
        }
    }

    pub async fn quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        max_accounts: Option<usize>,
        slippage_bps: u16,
        swap_mode: Option<SwapMode>,
        only_direct_routes: Option<bool>,
        excluded_dexes: Option<Vec<String>>,
    ) -> SdkResult<QuoteResponse> {
        let quote_request = QuoteRequest {
            input_mint,
            output_mint,
            amount,
            swap_mode,
            slippage_bps,
            platform_fee_bps: None,
            dexes: None,
            excluded_dexes,
            only_direct_routes,
            as_legacy_transaction: None,
            max_accounts,
            quote_type: None,
        };
        let query = serde_qs::to_string(&quote_request)
            .map_err(|e| SdkError::Generic(format!("failed to serialize: {e}")))?;
        let api_version_param = if self.url == "https://quote-api.jup.ag" {
            "/v6"
        } else {
            ""
        };

        let response = Client::new()
            .get(format!("{}{api_version_param}/quote?{query}", self.url))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response
                .json::<QuoteResponse>()
                .await
                .map_err(|e| SdkError::Generic(format!("failed to get json: {e}")))?)
        } else {
            Err(SdkError::Generic(format!(
                "Request status not ok: {}, body: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| SdkError::Generic(format!("failed to get text: {e}")))?
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::{jupiter::JupiterClient, types::Context, DriftClient};

    #[tokio::test]
    async fn test_quote() {
        let account_provider = 
        let drift_client = DriftClient::new(Context::DevNet, account_provider, wallet)
            .expect("construct drift client");
        let jupiter_swap_api_client = JupiterClient::new(None);
    }
}
