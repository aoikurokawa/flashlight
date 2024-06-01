use sdk::config::DriftEnv;

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

pub fn valid_rebalance_settled_pnl_threshold(amount: Option<f64>) -> bool {
    match amount {
        Some(a) if a >= 1.0 && a.fract() == 0.0 => true,
        _ => false,
    }
}

pub fn get_drift_priority_fee_endpoint(drift_env: DriftEnv) -> String {
    match drift_env {
        DriftEnv::Devnet => String::from(""),
        DriftEnv::MainnetBeta => String::from("https://dlob.drift.trade"),
    }
}
