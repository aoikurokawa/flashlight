use std::ops::{Add, Div, Mul, Sub};

use drift::{
    math::constants::{
        AMM_RESERVE_PRECISION, AMM_TO_QUOTE_PRECISION_RATIO, PEG_PRECISION, PERCENTAGE_PRECISION,
        PRICE_PRECISION,
    },
    state::perp_market::AMM,
};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, One, ToPrimitive};

use crate::{constants::PRICE_DIV_PEG, error::SdkError, types::SdkResult};

/// Helper function calculating adjust k cost
pub fn calculate_adjust_k_cost(amm: &AMM, numerator: u128, denomenator: u128) -> SdkResult<i128> {
    let x = BigInt::from(amm.base_asset_reserve);
    let y = BigInt::from(amm.quote_asset_reserve);

    let d = BigInt::from_i128(amm.base_asset_amount_with_amm).ok_or(SdkError::NumBigintError(
        "calculate_adjust_k_cost.amm_base_asset_amount_with_amm".to_string(),
    ))?;
    let q = BigInt::from(amm.peg_multiplier);

    let quote_scale = &y * &d * &q;

    let p = BigInt::from(numerator * PRICE_PRECISION / denomenator);
    let percentage_precision = BigInt::from(PERCENTAGE_PRECISION);
    let cost = quote_scale
        .clone()
        .mul(&percentage_precision)
        .mul(&percentage_precision)
        .div(&x + &d)
        .checked_sub(
            &quote_scale
                .mul(p.clone())
                .mul(&percentage_precision)
                .mul(&percentage_precision)
                .div(PRICE_PRECISION)
                .div(x.mul(p).div(PRICE_PRECISION).add(d)),
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

pub fn calculate_budget_peg(amm: &AMM, budget: i128, target_price: u64) -> SdkResult<u128> {
    let quote_asset_reserve = BigInt::from(amm.quote_asset_reserve);
    let terminal_quote_asset_reserve = BigInt::from(amm.terminal_quote_asset_reserve);

    let per_peg_cost = quote_asset_reserve
        .checked_sub(&terminal_quote_asset_reserve)
        .ok_or(SdkError::NumBigintError(
            "calculate_budget_peg.quote_asset_reserve".to_string(),
        ))?
        .div(BigInt::from(AMM_RESERVE_PRECISION.div(PRICE_PRECISION)));

    let per_peg_cost = if per_peg_cost > BigInt::ZERO {
        per_peg_cost + BigInt::one()
    } else {
        per_peg_cost - BigInt::one()
    };

    let target_price = BigInt::from(target_price);
    let target_peg = target_price
        .mul(amm.base_asset_reserve)
        .div(amm.quote_asset_reserve)
        .div(PRICE_DIV_PEG);

    let peg_change_direction = target_peg.clone().sub(BigInt::from(amm.peg_multiplier));

    let use_target_peg = (per_peg_cost < BigInt::ZERO && peg_change_direction > BigInt::ZERO)
        || (per_peg_cost > BigInt::ZERO && peg_change_direction < BigInt::ZERO);

    if per_peg_cost == BigInt::ZERO || use_target_peg {
        let target_peg = target_peg.to_u128().ok_or(SdkError::NumBigintError(
            "calculate_budget_peg.target_peg".to_string(),
        ))?;
        return Ok(target_peg);
    }

    let budget = BigInt::from_i128(budget).unwrap();
    let budget_delta_peg = budget.mul(PEG_PRECISION).div(per_peg_cost);
    let max = std::cmp::max(
        BigInt::one(),
        BigInt::from(amm.peg_multiplier).add(budget_delta_peg),
    );

    Ok(max.to_u128().ok_or(SdkError::NumBigintError(
        "calculate_budget_peg.max".to_string(),
    ))?)
}
