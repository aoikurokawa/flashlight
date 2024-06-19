use solana_sdk::pubkey::Pubkey;

use crate::{
    drift_client::DriftClient,
    priority_fee::types::DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS,
    types::{SdkError, SdkResult},
    AccountProvider,
};

use super::{
    average_over_slots_strategy::AverageOverSlotsStrategy,
    drift_priority_fee_method::{fetch_drift_priority_fee, DriftMarketInfo},
    helius_priority_fee_method::{
        fetch_helius_priority_fee, HeliusPriorityFeeLevels, HeliusPriorityLevel,
    },
    max_over_slots_strategy::MaxOverSlotsStrategy,
    solana_priority_fee_method::fetch_solana_priority_fee,
    types::{
        PriorityFeeMethod, PriorityFeeResponse, PriorityFeeStrategy, PriorityFeeSubscriberConfig,
    },
};

pub struct PriorityFeeSubscriber<T: AccountProvider> {
    // connection: Connection,
    drift_client: Option<DriftClient<T>>,
    frequency_ms: u64,
    addresses: Vec<Pubkey>,
    drift_markets: Option<Vec<DriftMarketInfo>>,
    custom_strategy: Option<Box<dyn PriorityFeeStrategy>>,
    average_strategy: AverageOverSlotsStrategy,
    max_strategy: MaxOverSlotsStrategy,
    priority_fee_method: PriorityFeeMethod,
    lookback_distance: u8,
    max_fee_micro_lamports: Option<u64>,
    priority_fee_multiplier: Option<f64>,

    drift_priority_fee_endpoint: Option<String>,
    helius_rpc_url: Option<String>,
    last_helius_sample: Option<HeliusPriorityFeeLevels>,

    // interval_id?: ReturnType<typeof setTimeout>;
    latest_priority_fee: u64,
    last_custom_strategy_result: u64,
    last_avg_strategy_result: u64,
    last_max_strategy_result: u64,
    last_slot_seen: u64,
}

impl<T: AccountProvider> PriorityFeeSubscriber<T> {
    pub fn new(config: PriorityFeeSubscriberConfig<T>) -> SdkResult<Self> {
        let drift_client = config.drift_client;
        let frequency_ms = match config.frequency_ms {
            Some(ms) => ms,
            None => DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS,
        };

        let addresses = match config.addresses {
            Some(keys) => keys,
            None => vec![],
        };

        let average_strategy = AverageOverSlotsStrategy;
        let custom_strategy = match config.custom_strategy {
            Some(strategy) => strategy,
            None => Box::new(average_strategy.clone()),
        };

        let lookback_distance = config.slots_to_check.unwrap_or(50);

        let mut priority_fee_method = None;
        let mut helius_rpc_url = None;
        let mut drift_priority_fee_endpoint = None;
        if let Some(method) = config.priority_fee_method {
            priority_fee_method = Some(method.clone());

            if method == PriorityFeeMethod::Helius {
                match config.helius_rpc_url {
                    None => {
                        if let Some(ref client) = drift_client {
                            if client
                                .backend
                                .account_provider
                                .endpoint()
                                .contains("helius")
                            {
                                helius_rpc_url = Some(client.backend.account_provider.endpoint());
                            } else {
                                return Err(SdkError::Generic("Connection must be helius, or helius_rpc_url must be provided to use PriorityFeeMethod::Helius".to_string()));
                            }
                        }
                    }
                    Some(rpc) => {
                        helius_rpc_url = Some(rpc);
                    }
                }
            } else if method == PriorityFeeMethod::Drift {
                drift_priority_fee_endpoint = config.drift_priority_fee_endpoint;
            }
        }

        if priority_fee_method == Some(PriorityFeeMethod::Solana) && drift_client.is_none() {
            return Err(SdkError::Generic(
                "connection must be provided to use SOLANA priority fee API".to_string(),
            ));
        }

        let priority_fee_multiplier = config.priority_fee_multiplier.unwrap_or(1.0);

        Ok(Self {
            drift_client,
            frequency_ms,
            addresses,
            drift_markets: config.drift_markets,
            custom_strategy: Some(custom_strategy),
            average_strategy,
            max_strategy: MaxOverSlotsStrategy {},
            priority_fee_method: PriorityFeeMethod::Solana,
            lookback_distance,
            max_fee_micro_lamports: config.max_fee_micro_lamports,
            priority_fee_multiplier: Some(priority_fee_multiplier),
            drift_priority_fee_endpoint,
            helius_rpc_url,
            last_helius_sample: None,
            latest_priority_fee: 0,
            last_custom_strategy_result: 0,
            last_avg_strategy_result: 0,
            last_max_strategy_result: 0,
            last_slot_seen: 0,
        })
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        self.load().await?;

        Ok(())
    }

    async fn load_for_solana(&mut self) -> SdkResult<()> {
        match &self.drift_client {
            Some(client) => {
                let samples =
                    fetch_solana_priority_fee(client, self.lookback_distance, &self.addresses)
                        .await?;

                if let Some(first) = samples.first() {
                    self.latest_priority_fee = first.prioritization_fee;
                    self.last_slot_seen = first.slot;

                    self.last_avg_strategy_result = self
                        .average_strategy
                        .calculate(PriorityFeeResponse::Solana(&samples));
                    self.last_max_strategy_result = self
                        .max_strategy
                        .calculate(PriorityFeeResponse::Solana(&samples));

                    if let Some(custom_strategy) = &self.custom_strategy {
                        self.last_custom_strategy_result =
                            custom_strategy.calculate(PriorityFeeResponse::Solana(&samples));
                    }
                }

                Ok(())
            }
            None => Err(SdkError::Generic(
                "Could not find the drift client".to_string(),
            )),
        }
    }

    async fn load_for_helius(&mut self) -> SdkResult<()> {
        match &self.helius_rpc_url {
            Some(helius_rpc_url) => {
                let result = fetch_helius_priority_fee(
                    helius_rpc_url,
                    self.lookback_distance,
                    &self.addresses,
                )
                .await;

                match result {
                    Ok(res) => {
                        self.last_helius_sample = res.result.priority_fee_levels.clone();

                        if let Some(sample) = &self.last_helius_sample {
                            self.last_avg_strategy_result =
                                *sample.0.get(&HeliusPriorityLevel::Medium).unwrap();

                            self.last_max_strategy_result =
                                *sample.0.get(&HeliusPriorityLevel::UnsafeMax).unwrap();
                        }

                        if let Some(custom_strategy) = &self.custom_strategy {
                            self.last_custom_strategy_result =
                                custom_strategy.calculate(PriorityFeeResponse::Helius(res));
                        }
                    }
                    Err(_e) => {
                        self.last_helius_sample = None;
                    }
                }
                Ok(())
            }

            None => Err(SdkError::Generic(
                "Could not find helius rpc url".to_string(),
            )),
        }
    }

    async fn load_for_drift(&mut self) -> SdkResult<()> {
        match &self.drift_priority_fee_endpoint {
            Some(endpoint) => {
                if let Some(drift_market) = &self.drift_markets {
                    let market_types: Vec<&str> = drift_market
                        .iter()
                        .map(|market| market.market_type.as_str())
                        .collect();
                    let market_indexes: Vec<u16> = drift_market
                        .iter()
                        .map(|market| market.market_index)
                        .collect();
                    let sample =
                        fetch_drift_priority_fee(endpoint, &market_types, &market_indexes).await?;

                    if !sample.0.is_empty() {
                        if let Some(sample) = &self.last_helius_sample {
                            self.last_avg_strategy_result =
                                *sample.0.get(&HeliusPriorityLevel::Medium).unwrap();

                            self.last_max_strategy_result =
                                *sample.0.get(&HeliusPriorityLevel::UnsafeMax).unwrap();
                        }

                        if let Some(custom_strategy) = &self.custom_strategy {
                            self.last_custom_strategy_result =
                                custom_strategy.calculate(PriorityFeeResponse::Drift(sample));
                        }
                    }
                }

                Ok(())
            }

            None => Err(SdkError::Generic(
                "Could not find drift priority fee endpoint".to_string(),
            )),
        }
    }

    pub fn get_max_priority_fee(&self) -> Option<u64> {
        self.max_fee_micro_lamports
    }

    pub fn update_max_priority_fee(&mut self, new_max_fee: Option<u64>) {
        self.max_fee_micro_lamports = new_max_fee;
    }

    pub fn get_priority_fee_multiplier(&self) -> f64 {
        self.priority_fee_multiplier.unwrap_or(1.0)
    }

    pub fn update_priority_fee_multiplier(&mut self, new_priority_fee_multiplier: Option<f64>) {
        self.priority_fee_multiplier = new_priority_fee_multiplier
    }

    pub fn update_custom_strategy(&mut self, new_strategy: Option<Box<dyn PriorityFeeStrategy>>) {
        self.custom_strategy = new_strategy;
    }

    pub fn get_helius_priority_fee_level(&self, level: Option<HeliusPriorityLevel>) -> u64 {
        let level = match level {
            Some(priority_level) => priority_level,
            None => HeliusPriorityLevel::Medium,
        };

        match &self.last_helius_sample {
            Some(helius_sample) => {
                let last_helius_sample = helius_sample.0.get(&level).unwrap();

                match &self.max_fee_micro_lamports {
                    Some(micro_lamports) => *std::cmp::min(micro_lamports, last_helius_sample),
                    None => *last_helius_sample,
                }
            }
            None => 0,
        }
    }

    pub fn get_custom_strategy_result(&self) -> f64 {
        let result = self.last_custom_strategy_result as f64 * self.get_priority_fee_multiplier();

        match self.max_fee_micro_lamports {
            Some(max_fee_micro_lamports) => (max_fee_micro_lamports as f64).min(result),
            None => result,
        }
    }

    pub fn get_avg_strategy_result(&self) -> f64 {
        let result = self.last_avg_strategy_result as f64 * self.get_priority_fee_multiplier();

        match self.max_fee_micro_lamports {
            Some(max_fee_micro_lamports) => (max_fee_micro_lamports as f64).min(result),
            None => result,
        }
    }

    pub fn get_max_strategy_result(&self) -> f64 {
        let result = self.last_max_strategy_result as f64 * self.get_priority_fee_multiplier();

        match self.max_fee_micro_lamports {
            Some(max_fee_micro_lamports) => (max_fee_micro_lamports as f64).min(result),
            None => result,
        }
    }

    pub async fn load(&mut self) -> SdkResult<()> {
        match self.priority_fee_method {
            PriorityFeeMethod::Solana => self.load_for_solana().await?,
            PriorityFeeMethod::Helius => self.load_for_helius().await?,
            PriorityFeeMethod::Drift => self.load_for_drift().await?,
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self) {}

    pub fn update_addresses(&mut self, addresses: &[Pubkey]) {
        self.addresses = addresses.to_vec();
    }

    pub fn update_market_type_and_index(&mut self, drift_markets: &[DriftMarketInfo]) {
        self.drift_markets = Some(drift_markets.to_vec());
    }
}
