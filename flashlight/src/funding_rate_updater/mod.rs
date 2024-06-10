use std::{
    collections::HashMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use drift::{
    math::helpers::on_the_hour_update,
    state::{paused_operations::PerpOperation, perp_market::MarketStatus},
};
use log::{error, info, warn};
use sdk::{
    config::DriftEnv,
    constants::perp_markets::read_perp_markets,
    drift_client::DriftClient,
    priority_fee::{
        drift_priority_fee_method::DriftMarketInfo,
        helius_priority_fee_method::HeliusPriorityLevel,
        priority_fee_subscriber_map::PriorityFeeSubscriberMap,
        types::PriorityFeeSubscriberMapConfig,
    },
    types::SdkResult,
    AccountProvider,
};
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    compute_budget::ComputeBudgetInstruction, instruction::InstructionError, pubkey::Pubkey,
    transaction::TransactionError,
};
use tokio::{sync::oneshot, task::JoinHandle, time::interval, time::Duration};

use crate::{
    config::BaseBotConfig,
    util::{
        get_drift_priority_fee_endpoint, simulate_and_get_tx_with_cus,
        SimulateAndGetTxWithCUsParams,
    },
};

const ERROR_CODES_TO_SUPPRESS: &[u32] = &[
    6040, 6251, // FundingWasNotUpdated
    6096, // AMMNotUpdatedInSameSlot
];

const ERROR_CODES_CAN_RETRY: &[u32] = &[
    6096, // AMMNotUpdatedInSameSlot
];

const CU_EST_MULTIPLIER: f64 = 1.4;

pub struct FundingRateUpdaterBot<T: AccountProvider, U> {
    name: String,
    dry_run: bool,
    run_once: bool,
    default_interval_ms: u64,

    drift_client: DriftClient<T, U>,
    interval_tx: Option<oneshot::Sender<()>>,
    interval_handles: Option<JoinHandle<()>>,
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
            interval_tx: None,
            interval_handles: None,
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

    pub async fn reset(&mut self) -> Result<(), String> {
        if let Some(interval_tx) = self.interval_tx.take() {
            interval_tx
                .send(())
                .map_err(|_| String::from("failed to send oneshot channel"))?;

            self.interval_handles = None;
        }

        Ok(())
    }

    pub async fn start_interval_loop(&mut self, interval_ms: u64) -> Result<(), String> {
        let (interval_tx, mut interval_rx) = oneshot::channel();
        self.interval_tx = Some(interval_tx);

        let mut interval = interval(Duration::from_millis(interval_ms));
        info!("{} Bot started! run_once {}", self.name, self.run_once);

        if self.run_once {
            self.try_update_funding_rate().await?;
        } else {
            self.try_update_funding_rate().await?;
            self.interval_handles = Some(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            println!("Inteval tick");
                        }
                        _ = &mut interval_rx => {
                            break;
                        }
                    }
                }
            }));
        }

        Ok(())
    }

    pub async fn try_update_funding_rate(&mut self) -> Result<(), String> {
        if self.in_progress {
            info!(
                "{} UpdateFundingReate already in progress, skipping...",
                self.name
            );
            return Ok(());
        }

        let _start = Instant::now();
        self.in_progress = true;

        let mut perp_market_and_oracle_data = HashMap::new();

        let perp_market_accounts = self.drift_client.get_perp_market_accounts();
        for market_account in perp_market_accounts {
            perp_market_and_oracle_data.insert(market_account.market_index, market_account);
        }

        for (_index, perp_market) in perp_market_and_oracle_data {
            if perp_market.status == MarketStatus::Initialized {
                info!(
                    "{} Skipping perp market {} because market status = {:?}",
                    self.name, perp_market.market_index, perp_market.status
                );
                continue;
            }

            let funding_paused = perp_market.is_operation_paused(PerpOperation::UpdateFunding);
            if funding_paused {
                let market_str =
                    String::from_utf8(perp_market.name.to_vec()).map_err(|e| e.to_string())?;
                warn!(
                    "{} Update funding paused for market: {} {}, skipping",
                    self.name, perp_market.market_index, market_str
                );
                continue;
            }

            if perp_market.amm.funding_period == 0 {
                info!(
                    "{} Perp market {}: AMM funding period is 0, skipping",
                    self.name, perp_market.market_index
                );
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
                "{} Perp market {}: time_remaining_til_update={}",
                self.name, perp_market.market_index, time_remaining_til_update
            );

            if time_remaining_til_update <= 0 {
                info!("{} Perp market {} last_funding_rate_ts: {}, funding_period: {}, last_funding_rate: {} vs. currTs: {current_ts}", 
                    self.name,
                    perp_market.market_index,
                    perp_market.amm.last_funding_rate_ts,
                    perp_market.amm.funding_period,
                    perp_market.amm.last_funding_rate_ts + perp_market.amm.funding_period
                );

                self.send_with_retry(perp_market.market_index, &perp_market.amm.oracle)
                    .await?;
            }
        }

        Ok(())
    }

    async fn send_with_retry(&self, market_index: u16, oracle: &Pubkey) -> Result<(), String> {
        let pfs = self
            .priority_fee_subscriber_map
            .get_priority_fees("perp", market_index as u64);
        let mut micro_lamports = 10_000;
        if let Some(pfs) = pfs {
            if let Some(lamport) = pfs.priority_fee_level.get(&HeliusPriorityLevel::Medium) {
                micro_lamports = *lamport;
            }
        }

        let max_retries = 30;
        for i in 0..max_retries {
            info!(
                "{} Funding rate update on market {market_index}, attempt: {}/{max_retries}",
                self.name,
                i + 1
            );

            let result = self.send_txs(micro_lamports, market_index, oracle).await?;
            // success
            if result.0 {
                break;
            }
            // can retry
            if result.1 {
                info!("{} Retrying market {market_index} in 1s...", self.name);
                continue;
            } else {
                break;
            }
        }

        Ok(())
    }

    async fn send_txs(
        &self,
        micro_lamports: u64,
        market_index: u16,
        oracle: &Pubkey,
    ) -> Result<(bool, bool), String> {
        let mut ixs = Vec::new();
        ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
        ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
            micro_lamports,
        ));

        let ix = self
            .drift_client
            .get_update_funding_rate_ix(market_index, oracle)
            .await
            .expect("failed to get instruction of update_funding_rate");
        ixs.push(ix);

        let recent_blockhash = self
            .drift_client
            .backend
            .rpc_client
            .get_latest_blockhash()
            .await
            .expect("get recent blockhash");
        let lookup_table_account = if let Some(lookup) = &self.lookup_table_account {
            lookup.clone()
        } else {
            self.drift_client
                .fetch_market_lookup_table_account()
                .clone()
        };
        let sim_result = simulate_and_get_tx_with_cus(&mut SimulateAndGetTxWithCUsParams {
            connection: self.drift_client.backend.rpc_client.clone(),
            payer: self.drift_client.wallet.signer.clone(),
            lookup_table_accounts: vec![lookup_table_account.clone()],
            ixs: ixs.into(),
            cu_limit_multiplier: Some(CU_EST_MULTIPLIER),
            do_simulation: Some(true),
            recent_blockhash: Some(recent_blockhash),
            dump_tx: None,
        })
        .await?;

        info!(
            "{} UpdateFundingRate estimated {} CUs for market: {market_index}",
            self.name, sim_result.cu_estimate
        );

        if let Some(sim_error) = sim_result.sim_error {
            if let TransactionError::InstructionError(code, e) = sim_error {
                if let InstructionError::Custom(custom_err) = e {
                    if ERROR_CODES_TO_SUPPRESS.contains(&custom_err) {
                        error!("{} Sim error (suppressed) on market: {market_index}, Error Code: {code} {e}", self.name);
                    } else {
                        error!("{} Sim error (not suppressed) on market: {market_index}, Error Code: {code} {e}", self.name);
                    }
                }

                if let InstructionError::Custom(_custom_err) = e {
                    return Ok((false, true));
                } else {
                    return Ok((false, false));
                }
            }
        }

        let send_tx_start = Instant::now();
        let tx_sig = self
            .drift_client
            .sign_and_send(sim_result.tx.message, false)
            .await
            .map_err(|e| e.to_string())?;

        info!(
            "{} UpdateFundingRate for market: {market_index}, tx sent in {}: https://solana.fm/tx/{}",
            self.name,
            send_tx_start.elapsed().as_millis(),
            tx_sig.to_string()
        );

        Ok((true, true))
    }
}
