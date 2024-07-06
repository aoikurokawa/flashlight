use std::ops::{Add, Div, Mul, Sub};

use drift::{
    math::{
        bn::U192,
        constants::{
            AMM_RESERVE_PRECISION, AMM_TO_QUOTE_PRECISION_RATIO, PEG_PRECISION,
            PERCENTAGE_PRECISION, PRICE_PRECISION,
        },
        safe_math::SafeMath,
    },
    state::perp_market::AMM,
};

use crate::{constants::PRICE_DIV_PEG, types::SdkResult};

/// Helper function calculating adjust k cost
pub fn calculate_adjust_k_cost(amm: &AMM, numerator: u128, denomenator: u128) -> SdkResult<i128> {
    let x = U192::from(amm.base_asset_reserve);
    let y = U192::from(amm.quote_asset_reserve);

    let d = U192::from(amm.base_asset_amount_with_amm);
    let q = U192::from(amm.peg_multiplier);

    let quote_scale = y * d * q;

    let p = U192::from(numerator * PRICE_PRECISION / denomenator);

    let cost = quote_scale
        .mul(U192::from(PERCENTAGE_PRECISION))
        .mul(U192::from(PERCENTAGE_PRECISION))
        .div(x.add(d))
        .sub(
            quote_scale
                .mul(p)
                .mul(U192::from(PERCENTAGE_PRECISION))
                .mul(U192::from(PERCENTAGE_PRECISION))
                .div(PRICE_PRECISION)
                .div(x.mul(p).div(PRICE_PRECISION).add(d)),
        )
        .div(PERCENTAGE_PRECISION)
        .div(PERCENTAGE_PRECISION)
        .div(AMM_TO_QUOTE_PRECISION_RATIO)
        .div(PEG_PRECISION);

    let cost = cost.try_to_u128()? as i128;
    Ok(cost.mul(-1))
}

pub fn calculate_budget_peg(amm: &AMM, budget: i128, target_price: u128) -> u128 {
    let per_peg_cost = amm
        .quote_asset_reserve
        .sub(amm.terminal_quote_asset_reserve)
        .div(AMM_RESERVE_PRECISION.div(PRICE_PRECISION)) as i128;

    let per_peg_cost = if per_peg_cost > 0 {
        per_peg_cost + 1
    } else {
        per_peg_cost - 1
    };

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
