pub fn valid_minimum_gas_amount(amount: Option<f64>) -> bool {
    if let Some(amount) = amount {
        if amount < 0.0 {
            return false;
        }
    }

    return true;
}
