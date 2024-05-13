use std::{
    ops::{Add, Div, Mul, Sub},
    time::{SystemTime, UNIX_EPOCH},
};

use drift::{
    controller::{amm::SwapDirection, position::PositionDirection},
    math::{
        amm::{calculate_price, calculate_swap_output},
        amm_spread::{calculate_inventory_liquidity_ratio, calculate_reference_price_offset},
        constants::{
            AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO, BID_ASK_SPREAD_PRECISION, PERCENTAGE_PRECISION,
        },
        repeg::{calculate_peg_from_target_price, calculate_repeg_cost},
    },
    state::{oracle::OraclePriceData, perp_market::AMM, user::AssetType},
};

use crate::{
    math::{repeg::calculate_budget_peg, util::sig_num},
    types::{SdkError, SdkResult},
};

use super::{
    oracle::{calculate_live_oracle_std, get_new_oracle_conf_pct},
    repeg::calculate_adjust_k_cost,
};

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

pub fn calculate_new_amm(
    amm: &AMM,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<(i128, u128, u128, u128)> {
    let mut pk_number = 1;
    let mut pk_denom = 1;

    let (target_price, mut new_peg, budget, _check_lower_bound) =
        calculate_optimal_peg_and_budget(amm, oracle_price_data)?;
    let mut pre_peg_cost = calculate_repeg_cost(amm, new_peg)?;

    if pre_peg_cost >= budget && pre_peg_cost > 0 {
        pk_number = 999;
        pk_denom = 1000;

        let deficit_madeup = calculate_adjust_k_cost(amm, pk_number, pk_denom);
        assert!(deficit_madeup <= 0);

        pre_peg_cost = budget + deficit_madeup.abs();
        let mut new_amm = amm.clone();
        new_amm.base_asset_reserve = new_amm.base_asset_reserve.mul(pk_number).div(pk_denom);
        new_amm.sqrt_k = new_amm.sqrt_k.mul(new_amm.sqrt_k);
        let invariant = new_amm.sqrt_k.mul(new_amm.sqrt_k);
        new_amm.quote_asset_reserve = invariant.div(new_amm.base_asset_reserve);
        let direction_to_close = if amm.base_asset_amount_with_amm > 0 {
            PositionDirection::Short
        } else {
            PositionDirection::Long
        };

        let swap_direction = get_swap_direction(AssetType::Base, direction_to_close);
        let (new_quote_asset_reserve, _new_base_asset_reserve) = calculate_amm_reserve_after_swap(
            &new_amm,
            AssetType::Base,
            new_amm.base_asset_amount_with_amm.abs(),
            swap_direction,
        )?;

        new_amm.terminal_quote_asset_reserve = new_quote_asset_reserve;
        new_peg = calculate_budget_peg(&new_amm, pre_peg_cost, target_price);
        pre_peg_cost = calculate_repeg_cost(&new_amm, new_peg)?;
    }

    Ok((pre_peg_cost, pk_number, pk_denom, new_peg))
}

pub fn calculate_updated_amm(amm: &AMM, oracle_price_data: &OraclePriceData) -> SdkResult<AMM> {
    if amm.curve_update_intensity == 0 {
        return Ok(*amm);
    }

    let mut new_amm = amm.clone();
    let (prepeg_cost, pk_number, pk_denom, new_peg) =
        calculate_new_amm(&new_amm, oracle_price_data)?;

    new_amm.base_asset_reserve = new_amm.base_asset_reserve.mul(pk_number).div(pk_denom);
    new_amm.sqrt_k = new_amm.sqrt_k.mul(pk_number).div(pk_denom);
    let invariant = new_amm.sqrt_k.mul(new_amm.sqrt_k);
    new_amm.quote_asset_reserve = invariant.div(new_amm.base_asset_reserve);
    new_amm.peg_multiplier = new_peg;

    let direction_to_close = if amm.base_asset_amount_with_amm > 0 {
        PositionDirection::Short
    } else {
        PositionDirection::Long
    };

    let swap_direction = get_swap_direction(AssetType::Base, direction_to_close);
    let (new_quote_asset_reserve, _new_base_asset_reserve) = calculate_amm_reserve_after_swap(
        &new_amm,
        AssetType::Base,
        amm.base_asset_amount_with_amm.abs(),
        swap_direction,
    )?;

    new_amm.terminal_quote_asset_reserve = new_quote_asset_reserve;

    new_amm.total_fee_minus_distributions = new_amm.total_fee_minus_distributions.sub(prepeg_cost);
    new_amm.net_revenue_since_last_funding =
        new_amm.net_revenue_since_last_funding - prepeg_cost as i64;

    Ok(new_amm)
}

/// Calculates what the amm reserves would be after swapping a quote or base asset amount.
pub fn calculate_amm_reserve_after_swap(
    amm: &AMM,
    input_asset_type: AssetType,
    swap_amount: i128,
    swap_direction: SwapDirection,
) -> SdkResult<(u128, u128)> {
    assert!(swap_amount >= 0, "swap_amount must be greater than 0");

    let mut swap_amount = swap_amount as u128;
    match input_asset_type {
        AssetType::Quote => {
            swap_amount = swap_amount
                .mul(AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO)
                .div(amm.peg_multiplier);

            Ok(calculate_swap_output(
                amm.quote_asset_reserve,
                swap_amount,
                swap_direction,
                amm.sqrt_k.mul(amm.sqrt_k),
            )?)
        }
        AssetType::Base => Ok(calculate_swap_output(
            amm.base_asset_reserve,
            swap_amount,
            swap_direction,
            amm.sqrt_k.mul(amm.sqrt_k),
        )?),
    }
}

pub fn calculate_spread_bn() {}

pub fn calculate_spread(
    amm: &AMM,
    oracle_price_data: Option<&OraclePriceData>,
    now: Option<i128>,
    reserve_price: Option<u128>,
) -> SdkResult<(u16, u16)> {
    let reserve_price = match reserve_price {
        Some(price) => price,
        None => calculate_price(
            amm.base_asset_reserve,
            amm.quote_asset_reserve,
            amm.peg_multiplier,
        )? as u128,
    };

    let target_price = match oracle_price_data {
        Some(data) => data.price as u128,
        None => reserve_price,
    };
    let target_mark_spread_pct = reserve_price
        .sub(target_price)
        .mul(BID_ASK_SPREAD_PRECISION as u128)
        .div(reserve_price);

    let now = match now {
        Some(time) => time,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i128,
    };

    if let Some(oracle_price_data) = oracle_price_data {
        let live_oracle_std = calculate_live_oracle_std(amm, oracle_price_data, now);
        let conf_interval_pct = get_new_oracle_conf_pct(amm, oracle_price_data, reserve_price, now);
    }

    let spreads = calculate_spread_bn();

    Ok((0, 0))
}

pub fn calculate_spread_reserves(
    amm: &AMM,
    oracle_price_data: &OraclePriceData,
    now: Option<u128>,
) -> SdkResult<()> {
    fn calculate_spread_reserve(
        spread: i128,
        direction: PositionDirection,
        amm: &AMM,
    ) -> (u128, u128) {
        if spread == 0 {
            return (amm.base_asset_reserve, amm.quote_asset_reserve);
        }
        let mut spread_fraction = spread / 2;

        if spread_fraction == 0 {
            spread_fraction = if spread >= 0 { 1 } else { -1 };
        }

        let quote_asset_reserve_delta = amm
            .quote_asset_reserve
            .div((BID_ASK_SPREAD_PRECISION as i128 / spread_fraction) as u128);

        let quote_asset_reserve = if quote_asset_reserve_delta >= 0 {
            amm.quote_asset_reserve + quote_asset_reserve_delta
        } else {
            amm.quote_asset_reserve - quote_asset_reserve_delta
        };

        let base_asset_reserve = amm.sqrt_k.mul(amm.sqrt_k).div(quote_asset_reserve);

        (base_asset_reserve, quote_asset_reserve)
    }

    let reserve_price = calculate_price(
        amm.base_asset_reserve,
        amm.quote_asset_reserve,
        amm.peg_multiplier,
    )?;

    let mut max_offset = 0;
    let mut reference_price_offset = 0;
    if amm.curve_update_intensity > 100 {
        max_offset = std::cmp::max(
            amm.max_spread as u128 / 5,
            (PERCENTAGE_PRECISION / 10000) * (amm.curve_update_intensity as u128 - 100),
        );
    }

    let liquidity_fraction = calculate_inventory_liquidity_ratio(
        amm.base_asset_amount_with_amm,
        amm.base_asset_reserve,
        amm.min_base_asset_reserve,
        amm.max_base_asset_reserve,
    )?;
    let liquidity_fraction_signed = liquidity_fraction.mul(sig_num(
        amm.base_asset_amount_with_amm
            .add(amm.base_asset_amount_per_lp),
    ));
    reference_price_offset = calculate_reference_price_offset(
        reserve_price,
        amm.last_24h_avg_funding_rate,
        liquidity_fraction_signed,
        0,
        amm.historical_oracle_data.last_oracle_price_twap_5min,
        amm.last_mark_price_twap_5min,
        amm.historical_oracle_data.last_oracle_price_twap,
        amm.last_mark_price_twap,
        max_offset as i64,
    )?;

    Ok(())
}

/// Translate long/shorting quote/base assert into amm operation
pub fn get_swap_direction(
    input_asset_type: AssetType,
    position_direction: PositionDirection,
) -> SwapDirection {
    match position_direction {
        PositionDirection::Long if input_asset_type == AssetType::Base => SwapDirection::Remove,
        PositionDirection::Short if input_asset_type == AssetType::Quote => SwapDirection::Remove,
        _ => SwapDirection::Add,
    }
}
