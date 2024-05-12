use drift::{
    math::{
        amm::calculate_price,
        constants::BID_ASK_SPREAD_PRECISION,
        repeg::{calculate_peg_from_target_price, calculate_repeg_cost},
    },
    state::{oracle::OraclePriceData, perp_market::AMM},
};

use crate::types::{SdkError, SdkResult};

pub fn calculate_optimal_peg_and_budget(
    amm: &AMM,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<(u128, u128, i128, bool)> {
    let reserve_price_before = calculate_price(
        amm.quote_asset_reserve,
        amm.base_asset_reserve,
        amm.peg_multiplier,
    )
    .map_err(|e| SdkError::MathError(format!("Error Code: {e}")))?;
    let target_price = oracle_price_data.price;
    let new_peg = calculate_peg_from_target_price(
        target_price as u128,
        amm.base_asset_reserve,
        amm.quote_asset_reserve as u64,
    )
    .map_err(|e| SdkError::MathError(format!("Error Code: {e}")))?;
    let pre_peg_cost = calculate_repeg_cost(amm, new_peg)
        .map_err(|e| SdkError::MathError(format!("Error Code: {e}")))?;

    let total_fee_lb = amm.total_exchange_fee / 2;
    let budget = std::cmp::max(0, amm.total_fee_minus_distributions - total_fee_lb as i128);

    let mut new_target_price = 0;
    let mut new_optimal_peg = 0;
    let mut new_budget = 0;
    let mut check_lower_bound = true;
    if budget < pre_peg_cost {
        let half_max_price_spread =
            amm.max_spread as i64 / 2 * target_price / BID_ASK_SPREAD_PRECISION as i64;

        let target_price_gap = reserve_price_before as i64 - target_price;

        if target_price_gap.abs() > half_max_price_spread {
            let mark_adj = target_price_gap.abs() - half_max_price_spread;

            new_target_price = if target_price_gap < 0 {
                reserve_price_before as u128 + mark_adj as u128
            } else {
                reserve_price_before as u128 - mark_adj as u128
            };

            new_optimal_peg = calculate_peg_from_target_price(
                new_target_price,
                amm.base_asset_reserve,
                amm.quote_asset_reserve as u64,
            )
            .map_err(|e| SdkError::MathError(format!("Error Code: {e}")))?;

            new_budget = calculate_repeg_cost(amm, new_optimal_peg)
                .map_err(|e| SdkError::MathError(format!("Error Code: {e}")))?;
            check_lower_bound = false;

            return Ok((
                new_target_price,
                new_optimal_peg,
                new_budget,
                check_lower_bound,
            ));
        } else if amm.total_fee_minus_distributions < (amm.total_exchange_fee / 2) as i128 {
            check_lower_bound = false;
        }
    }

    return Ok((
        new_target_price,
        new_optimal_peg,
        new_budget,
        check_lower_bound,
    ));
}

pub fn calculate_new_amm(amm: &AMM, oracle_price_data: &OraclePriceData) -> SdkResult<()> {
    let pk_number = 1;
    let pk_denom = 1;

    let (target_price, new_peg, budget, _check_lower_bound) =
        calculate_optimal_peg_and_budget(amm, oracle_price_data)?;
    let pre_peg_cost = calculate_repeg_cost(amm, new_peg)?;

    if pre_peg_cost >= budget && pre_peg_cost > 0 {}

    Ok(())
}
