use solana_sdk::pubkey::Pubkey;

use crate::{
    priority_fee::types::DEFAULT_PRIORITY_FEE_MAP_FREQUENCY_MS,
    types::{SdkError, SdkResult},
    AccountProvider, DriftClient,
};

use super::{
    average_over_slots_strategy::AverageOverSlotsStrategy,
    drift_priority_fee_method::DriftMarketInfo,
    helius_priority_fee_method::HeliusPriorityFeeLevels,
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
    lookback_distance: u64,
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

        let lookback_distance = match config.slots_to_check {
            Some(x) => x,
            None => 50,
        };

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

        let priority_fee_multiplier = match config.priority_fee_multiplier {
            Some(fee_multiplier) => fee_multiplier,
            None => 1.0,
        };

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

    pub async fn subscribe(&mut self) {
        // if self.
    }

    async fn load_for_solana(&mut self) -> SdkResult<()> {
        match &self.drift_client {
            Some(client) => {
                let samples =
                    fetch_solana_priority_fee(&client, self.lookback_distance, &self.addresses)
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
        match &self.drift_client {
            Some(client) => {
                let samples =
                    fetch_solana_priority_fee(&client, self.lookback_distance, &self.addresses)
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

    pub async fn load(&self) {}
}
