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
