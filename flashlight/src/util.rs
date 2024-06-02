use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use base64::Engine;
use log::info;
use sdk::config::DriftEnv;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table_account::AddressLookupTableAccount,
    compute_budget::{ComputeBudgetInstruction, ID as ComputeBudgetProgramId},
    hash::Hash,
    instruction::Instruction,
    message::{v0::Message, VersionedMessage},
    signature::Keypair,
    signer::Signer,
    transaction::{TransactionError, VersionedTransaction},
};

pub fn valid_minimum_gas_amount(amount: Option<f64>) -> bool {
    if amount.is_none() {
        return false;
    }

    if let Some(amount) = amount {
        if amount < 0.0 {
            return false;
        }
    }

    return true;
}

pub fn is_set_compute_units_ix(ix: &Instruction) -> bool {
    if ix.program_id == ComputeBudgetProgramId && ix.data.first() == Some(&2) {
        return true;
    }

    false
}

fn get_versioned_transaction(
    payer: &Keypair,
    ixs: &[Instruction],
    lookup_table_accounts: &[AddressLookupTableAccount],
    recent_blockhash: Hash,
) -> Result<VersionedTransaction, String> {
    let message = Message::try_compile(
        &payer.pubkey(),
        ixs,
        lookup_table_accounts,
        recent_blockhash,
    )
    .map_err(|e| e.to_string())?;
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[payer])
        .map_err(|e| e.to_string())?;

    Ok(tx)
}

// const PLACEHOLDER_BLOCKHASH: &[u8] = b"Fdum64WVeej6DeL85REV9NvfSxEJNPZ74DBk7A8kTrKP";

pub struct SimulateAndGetTxWithCUsParams {
    pub connection: Arc<RpcClient>,
    pub payer: Arc<Keypair>,
    pub lookup_table_accounts: Vec<AddressLookupTableAccount>,

    /// instructions to simulate and create transaction from
    pub ixs: VecDeque<Instruction>,

    /// multiplier to apply to the estimated CU usage, default: 1.0
    pub cu_limit_multiplier: Option<f64>,

    /// set false to only create a tx without simulating for CU estimate
    pub do_simulation: Option<bool>,

    /// recentBlockhash to use in the final tx. If undefined, PLACEHOLDER_BLOCKHASH
    /// will be used for simulation, the final tx will have an empty blockhash so
    /// attempts to sign it will throw.
    pub recent_blockhash: Option<Hash>,

    /// set true to dump base64 transaction before and after simulating for CUs
    pub dump_tx: Option<bool>,
}

pub struct SimulateAndGetTxWithCUsResponse {
    pub cu_estimate: i64,
    pub sim_tx_logs: Option<Vec<String>>,
    pub sim_error: Option<TransactionError>,
    pub sim_tx_duration: Duration,
    pub tx: VersionedTransaction,
}

/// Simulates the instructions in order to determine how many CUs it needs,
/// applies `cuLimitMulitplier` to the estimate and inserts or modifies
/// the CU limit request ix.
///
/// If `recentBlockhash` is provided, it is used as is to generate the final
/// tx. If it is undefined, uses `PLACEHOLDER_BLOCKHASH` which is a valid
/// blockhash to perform simulation and removes it from the final tx. Signing
/// a tx without a valid blockhash will throw.
pub async fn simulate_and_get_tx_with_cus(
    params: &mut SimulateAndGetTxWithCUsParams,
) -> Result<SimulateAndGetTxWithCUsResponse, String> {
    if params.ixs.is_empty() {
        return Err("cannot simulate empty tx".to_string());
    }

    let mut set_cu_limit_ix_idx = -1;
    for (index, ix) in params.ixs.iter().enumerate() {
        if is_set_compute_units_ix(ix) {
            set_cu_limit_ix_idx = index as isize;
            break;
        }
    }

    // if we don't have a set CU limit ix, add one to the beginning
    // otherwise the default CU limit for sim is 400k, which may be too low
    if set_cu_limit_ix_idx == -1 {
        params
            .ixs
            .push_front(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
        set_cu_limit_ix_idx = 0;
    }

    // let sim_tx_duration: Duration = 0;
    // let place_holder = Hash::new(PLACEHOLDER_BLOCKHASH);
    let tx = get_versioned_transaction(
        &params.payer,
        &Vec::from(params.ixs.clone()),
        &params.lookup_table_accounts,
        params.recent_blockhash.unwrap(),
    )?;
    if params.do_simulation.is_none() {
        return Ok(SimulateAndGetTxWithCUsResponse {
            cu_estimate: -1,
            sim_tx_logs: None,
            sim_error: None,
            sim_tx_duration: Duration::new(0, 0),
            tx,
        });
    }
    if params.dump_tx.is_some() {
        info!("===== Simulating The following transaction =====");
        let serizlied_tx =
            base64::engine::general_purpose::STANDARD.encode(&tx.message.serialize());
        info!("{}", serizlied_tx);
        info!("================================================");
    }

    let start = Instant::now();
    let resp = params
        .connection
        .simulate_transaction(&tx)
        .await
        .map_err(|e| format!("Failed to simulate transaction: {}", e.to_string()))?;
    info!("Response: {resp:?}");
    let sim_tx_duration = start.elapsed();

    let sim_tx_logs = resp.value.logs;
    let cu_estimate = match resp.value.units_consumed {
        Some(estimate) => estimate,
        None => {
            return Err(String::from(
                "Failed to get units comsumed from simulateTransaction",
            ))
        }
    };
    let cu_to_use = cu_estimate as f64 * (params.cu_limit_multiplier.unwrap_or(1.0));
    params.ixs[set_cu_limit_ix_idx as usize] =
        ComputeBudgetInstruction::set_compute_unit_limit(cu_to_use as u32);

    let recent_blockhash = params.recent_blockhash.unwrap();
    let mut tx_with_cus = get_versioned_transaction(
        &params.payer,
        &Vec::from(params.ixs.clone()),
        &params.lookup_table_accounts,
        recent_blockhash,
    )?;
    if params.dump_tx.is_some() {
        info!("== Simulation result, cuEstimate: {cu_estimate}, using: {cu_to_use}, blockhash: {recent_blockhash} ==");
        let serizlied_tx =
            base64::engine::general_purpose::STANDARD.encode(&tx.message.serialize());
        info!("{}", serizlied_tx);
        info!("================================================");
    }

    // strip out the placeholder blockhash so user doesn't try to send the tx.
    // sending a tx with placeholder blockhash will cause `blockhash not found error`
    // which is suppressed if flight checks are skipped.
    if params.recent_blockhash.is_none() {
        tx_with_cus.message.set_recent_blockhash(Hash::new(&[0]));
    }

    Ok(SimulateAndGetTxWithCUsResponse {
        cu_estimate: cu_estimate as i64,
        sim_tx_logs,
        sim_error: resp.value.err,
        sim_tx_duration,
        tx,
    })
}

pub fn get_drift_priority_fee_endpoint(drift_env: DriftEnv) -> String {
    match drift_env {
        DriftEnv::Devnet => String::from(""),
        DriftEnv::MainnetBeta => String::from("https://dlob.drift.trade"),
    }
}

pub fn valid_rebalance_settled_pnl_threshold(amount: Option<f64>) -> bool {
    match amount {
        Some(a) if a >= 1.0 && a.fract() == 0.0 => true,
        _ => false,
    }
}
