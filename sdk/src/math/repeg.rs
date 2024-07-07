use std::ops::{Add, Div, Mul, Sub};

use drift::{
    math::constants::{
        AMM_RESERVE_PRECISION, AMM_TO_QUOTE_PRECISION_RATIO, PEG_PRECISION, PERCENTAGE_PRECISION,
        PRICE_PRECISION,
    },
    state::perp_market::AMM,
};
use num_bigint::{BigInt, BigUint};
use num_traits::{CheckedSub, FromPrimitive, ToPrimitive};

use crate::{constants::PRICE_DIV_PEG, error::SdkError, types::SdkResult};

/// Helper function calculating adjust k cost
pub fn calculate_adjust_k_cost(amm: &AMM, numerator: u128, denomenator: u128) -> SdkResult<i128> {
    let x = BigUint::from(amm.base_asset_reserve);
    let y = BigUint::from(amm.quote_asset_reserve);

    let d = BigInt::from_i128(amm.base_asset_amount_with_amm).ok_or(SdkError::NumBigintError(
        "calculate_adjust_k_cost.amm_base_asset_amount_with_amm".to_string(),
    ))?;
    let q = BigUint::from(amm.peg_multiplier);

    let quote_scale = &y * &d.to_biguint().unwrap() * &q;

    let p = BigUint::from(numerator * PRICE_PRECISION / denomenator);

    let cost = quote_scale
        .clone()
        .mul(BigUint::from(PERCENTAGE_PRECISION))
        .mul(BigUint::from(PERCENTAGE_PRECISION))
        .div(&x + &d.to_biguint().unwrap())
        .checked_sub(
            &quote_scale
                .mul(p.clone())
                .mul(BigUint::from(PERCENTAGE_PRECISION))
                .mul(BigUint::from(PERCENTAGE_PRECISION))
                .div(PRICE_PRECISION)
                .div(x.mul(p).div(PRICE_PRECISION).add(d.to_biguint().unwrap())),
        )
        .ok_or(SdkError::NumBigintError(
            "calculate_adjust_k_cost.quote_scale".to_string(),
        ))?
        .div(PERCENTAGE_PRECISION)
        .div(PERCENTAGE_PRECISION)
        .div(AMM_TO_QUOTE_PRECISION_RATIO)
        .div(PEG_PRECISION);

    let cost = cost.to_i128().ok_or(SdkError::NumBigintError(
        "calculate_adjust_k_cost.cost".to_string(),
    ))?;
    Ok(cost.mul(-1))
}

pub fn calculate_budget_peg(amm: &AMM, budget: i128, target_price: u64) -> u128 {
    let per_peg_cost = amm
        .quote_asset_reserve
        .sub(amm.terminal_quote_asset_reserve)
        .div(AMM_RESERVE_PRECISION.div(PRICE_PRECISION)) as i128;

    let per_peg_cost = if per_peg_cost > 0 {
        per_peg_cost + 1
    } else {
        per_peg_cost - 1
    };

    let target_price = target_price as u128;
    let target_peg = target_price
        .mul(amm.base_asset_reserve)
        .div(amm.quote_asset_reserve)
        .div(PRICE_DIV_PEG);

    let peg_change_direction = target_peg.sub(amm.peg_multiplier) as i128;

    let use_target_peg = (per_peg_cost < 0 && peg_change_direction > 0)
        || (per_peg_cost > 0 && peg_change_direction < 0);

    if per_peg_cost == 0 || use_target_peg {
        return target_peg;
    }

    let budget = budget as u128;
    let budget_delta_peg = budget.mul(PEG_PRECISION).div(per_peg_cost as u128);
    std::cmp::max(1, amm.peg_multiplier.add(budget_delta_peg))
}
