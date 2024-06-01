use std::ops::{Add, Div, Mul, Sub};

use drift::{
    math::constants::{
        AMM_RESERVE_PRECISION, AMM_TO_QUOTE_PRECISION_RATIO, PEG_PRECISION, PERCENTAGE_PRECISION,
        PRICE_PRECISION,
    },
    state::perp_market::AMM,
};

use crate::constants::PRICE_DIV_PEG;

/// Helper function calculating adjust k cost
pub fn calculate_adjust_k_cost(amm: &AMM, numerator: u128, denomenator: u128) -> i128 {
    let x = amm.base_asset_reserve;
    let y = amm.quote_asset_reserve;

    let d = amm.base_asset_amount_with_amm as u128;
    let q = amm.peg_multiplier;

    let quote_scale = y * d * q;

    let p = numerator * PRICE_PRECISION / denomenator;

    let cost = quote_scale
        .mul(PERCENTAGE_PRECISION)
        .mul(PERCENTAGE_PRECISION)
        .div(x.add(d))
        .sub(
            quote_scale
                .mul(p)
                .mul(PERCENTAGE_PRECISION)
                .mul(PERCENTAGE_PRECISION)
                .div(PRICE_PRECISION)
                .div(x.mul(p).div(PRICE_PRECISION).add(d)),
        )
        .div(PERCENTAGE_PRECISION)
        .div(PERCENTAGE_PRECISION)
        .div(AMM_TO_QUOTE_PRECISION_RATIO)
        .div(PEG_PRECISION) as i128;

    cost.mul(-1)
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
