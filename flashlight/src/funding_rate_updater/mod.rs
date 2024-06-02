use std::{
    collections::HashMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use drift::{
    math::helpers::on_the_hour_update,
    state::{paused_operations::PerpOperation, perp_market::MarketStatus},
};
use log::{info, warn};
use sdk::{
    config::DriftEnv,
    constants::perp_markets::read_perp_markets,
    priority_fee::{
        drift_priority_fee_method::DriftMarketInfo,
        priority_fee_subscriber_map::PriorityFeeSubscriberMap,
        types::PriorityFeeSubscriberMapConfig,
    },
    types::{SdkError, SdkResult},
    AccountProvider, DriftClient,
};
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    compute_budget::ComputeBudgetInstruction,
};

use crate::{config::BaseBotConfig, util::get_drift_priority_fee_endpoint};

pub struct FundingRateUpdaterBot<T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    run_once: bool,
    default_interval_ms: u64,

    drift_client: DriftClient<T, U>,
    interval_ids: Vec<u64>,
    priority_fee_subscriber_map: PriorityFeeSubscriberMap,
    lookup_table_account: Option<AddressLookupTableAccount>,

    watchdog_timer_last_par_time: Instant,
    in_progress: bool,
}

impl<T: AccountProvider, U> FundingRateUpdaterBot<T, U> {
    pub fn new(drift_client: DriftClient<T, U>, config: BaseBotConfig) -> Self {
        let perp_markets = read_perp_markets(DriftEnv::Devnet);
        let drift_markets = perp_markets
            .iter()
            .map(|perp_market| DriftMarketInfo {
                market_type: "perp".to_string(),
                market_index: perp_market.market_index,
            })
            .collect();
        let priority_config = PriorityFeeSubscriberMapConfig {
            frequency_ms: Some(10_000),
            drift_markets: Some(drift_markets),
            drift_priority_fee_endpoint: get_drift_priority_fee_endpoint(DriftEnv::Devnet),
        };

        Self {
            name: config.bot_id,
            dry_run: config.dry_run,
            run_once: config.run_once.unwrap_or(false),
            default_interval_ms: 120000,
            drift_client,
            interval_ids: Vec::new(),
            priority_fee_subscriber_map: PriorityFeeSubscriberMap::new(priority_config),
            lookup_table_account: None,
            watchdog_timer_last_par_time: Instant::now(),
            in_progress: false,
        }
    }

    pub async fn init(&mut self) -> SdkResult<()> {
        self.priority_fee_subscriber_map.subscribe().await?;
        self.lookup_table_account = Some(self.drift_client.fetch_market_lookup_table_account());

        info!("{} inited", self.name);

        Ok(())
    }

    pub async fn reset(&mut self) {}

    pub async fn start_interval_loop(&mut self, interval_ms: u64) -> SdkResult<()> {
        info!("{} Bot started! run_once {}", self.name, self.run_once);

        if self.run_once {
            self.try_update_funding_rate().await?;
        } else {
            self.try_update_funding_rate().await?;
        }

        Ok(())
    }

    pub async fn try_update_funding_rate(&mut self) -> SdkResult<()> {
        if self.in_progress {
            info!(
                "{} UpdateFundingReate already in progress, skipping...",
                self.name
            );
            return Ok(());
        }

        let start = Instant::now();
        self.in_progress = true;

        let mut perp_market_and_oracle_data = HashMap::new();

        let perp_market_accounts = self.drift_client.get_perp_market_accounts();
        for market_account in perp_market_accounts {
            perp_market_and_oracle_data.insert(market_account.market_index, market_account);
        }

        for (index, perp_market) in perp_market_and_oracle_data {
            if perp_market.status == MarketStatus::Initialized {
                info!(
                    "{} Skipping perp market {} because market status = {:?}",
                    self.name, perp_market.market_index, perp_market.status
                );
                continue;
            }

            let funding_paused = perp_market.is_operation_paused(PerpOperation::UpdateFunding);
            if funding_paused {
                let market_str = String::from_utf8(perp_market.name.to_vec())
                    .map_err(|e| SdkError::Generic(e.to_string()))?;
                warn!(
                    "{} Update funding paused for market: {} {},  skipping",
                    self.name, perp_market.market_index, market_str
                );
                continue;
            }

            if perp_market.amm.funding_period == 0 {
                continue;
            }

            let current_ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64;

            let time_remaining_til_update = on_the_hour_update(
                current_ts,
                perp_market.amm.last_funding_rate_ts,
                perp_market.amm.funding_period,
            )
            .expect("");

            info!(
                "{} Perp market {} time_remaining_til_update={}",
                self.name, perp_market.market_index, time_remaining_til_update
            )
        }

        Ok(())
    }

    async fn send_txs(&self, micro_lamports: u64) -> (bool, bool) {
        let mut ixs = Vec::new();
        ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
        ixs.push(ComputeBudgetInstruction::set_compute_unit_price(micro_lamports));
        ixs.push(self.drift_client.get_update_funding_rate_ix());
        (true, true)
    }
}
