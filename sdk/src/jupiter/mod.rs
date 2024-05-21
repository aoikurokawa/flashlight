use std::{collections::HashMap, str::FromStr};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount, pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::{
    jupiter::serde_helpers::field_as_string,
    types::{SdkError, SdkResult},
};

use self::{
    swap::{SwapRequest, SwapResponse},
    transaction_config::TransactionConfig,
};

mod serde_helpers;
mod swap;
mod transaction_config;

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

pub struct JupiterClient {
    url: String,
    rpc_client: RpcClient,
    lookup_table_cache: HashMap<String, AddressLookupTableAccount>,
}

impl JupiterClient {
    pub fn new(rpc_client: RpcClient, url: Option<String>) -> Self {
        let url = match url {
            Some(url) => url,
            None => "https://quote-api.jup.ag".to_string(),
        };

        Self {
            url,
            rpc_client,
            lookup_table_cache: HashMap::new(),
        }
    }

    /// Get routes for a swap
    pub async fn get_quote(
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

    /// Get a swap transaction for quote
    pub async fn get_swap(
        &self,
        mut quote_response: QuoteResponse,
        user_public_key: Pubkey,
        slippage_bps: Option<u16>,
    ) -> SdkResult<VersionedTransaction> {
        let slippage_bps = match slippage_bps {
            Some(n) => n,
            None => 50,
        };
        let api_version_param = if self.url == "https://quote-api.jup.ag" {
            "/v6"
        } else {
            ""
        };

        quote_response.slippage_bps = slippage_bps;
        let swap_request = SwapRequest {
            user_public_key,
            quote_response,
            config: TransactionConfig::default(),
        };
        let response = Client::new()
            .post(format!("{}{api_version_param}/swap", self.url))
            .json(&swap_request)
            .send()
            .await?;

        if response.status().is_success() {
            let res = response
                .json::<SwapResponse>()
                .await
                .map_err(|e| SdkError::Generic(format!("failed to get json: {e}")))?;

            let versioned_transaction: VersionedTransaction =
                bincode::deserialize(&res.swap_transaction)
                    .map_err(|_e| SdkError::Deserializing)?;

            Ok(versioned_transaction)
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

    async fn get_lookup_table(&self, account_key: Pubkey) -> SdkResult<&AddressLookupTableAccount> {
        match self.lookup_table_cache.get(&account_key.to_string()) {
            Some(table_account) => Ok(table_account),
            None => Err(SdkError::Generic("Not found".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::pubkey;
    use solana_sdk::pubkey::Pubkey;

    use crate::jupiter::JupiterClient;

    const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
    const TEST_WALLET: Pubkey = pubkey!("2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm");

    #[tokio::test]
    async fn test_get_quote() {
        let rpc_client = RpcClient::new("".to_string());
        let jupiter_client = JupiterClient::new(rpc_client, None);

        // GET /quote
        let quote_response = jupiter_client
            .get_quote(
                USDC_MINT,
                NATIVE_MINT,
                1_000_000,
                None,
                50,
                None,
                None,
                None,
            )
            .await;

        assert!(quote_response.is_ok());
    }

    #[tokio::test]
    async fn test_get_swap() {
        let rpc_client = RpcClient::new("".to_string());
        let jupiter_client = JupiterClient::new(rpc_client, None);

        let quote_response = jupiter_client
            .get_quote(
                USDC_MINT,
                NATIVE_MINT,
                1_000_000,
                None,
                50,
                None,
                None,
                None,
            )
            .await
            .expect("failed to get quote");

        // GET /swap
        let swap_response = jupiter_client
            .get_swap(quote_response, TEST_WALLET, None)
            .await;

        assert!(swap_response.is_ok());
    }
}
