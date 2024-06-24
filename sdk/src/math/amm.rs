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
            AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO, BID_ASK_SPREAD_PRECISION, PEG_PRECISION,
            PERCENTAGE_PRECISION, PRICE_PRECISION,
        },
        orders::standardize_base_asset_amount,
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
    util::{clamp_bn, square_root_u128},
};

#[derive(Debug, Default)]
pub struct SpreadTerms {
    long_vol_spread: u128,
    short_vol_spread: u128,
    long_spread_w_ps: u128,
    short_spread_w_ps: u128,
    max_target_spread: u128,
    inventory_spread_scale: u128,
    long_spread_w_inv_scale: u128,
    short_spread_w_inv_scale: u128,
    effective_leverage: u128,
    effective_leverage_capped: u128,
    long_spread_w_el: u128,
    short_spread_w_el: u128,
    revenue_retreat_amount: u128,
    half_revenue_retreat_amount: u128,
    long_spread_w_rev_retreat: u128,
    short_spread_w_rev_retreat: u128,
    total_spread: u128,
    long_spread: u128,
    short_spread: u128,
}

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

    Ok((
        new_target_price,
        new_optimal_peg,
        new_budget,
        check_lower_bound,
    ))
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
        let mut new_amm = *amm;
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
        let (new_quote_asset_reserve, _new_base_asset_reserve) = calculate_amm_reserves_after_swap(
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

    let mut new_amm = *amm;
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
    let (new_quote_asset_reserve, _new_base_asset_reserve) = calculate_amm_reserves_after_swap(
        &new_amm,
        AssetType::Base,
        amm.base_asset_amount_with_amm.abs(),
        swap_direction,
    )?;

    new_amm.terminal_quote_asset_reserve = new_quote_asset_reserve;

    new_amm.total_fee_minus_distributions = new_amm.total_fee_minus_distributions.sub(prepeg_cost);
    new_amm.net_revenue_since_last_funding -= prepeg_cost as i64;

    Ok(new_amm)
}

/// Return `base_asset_reserve`, `quote_asset_reserve`, `sqrt_k`, `new_peg`
pub fn calculate_updated_amm_spread_reserves(
    amm: &AMM,
    direction: PositionDirection,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<(u128, u128, u128, u128)> {
    let new_amm = calculate_updated_amm(amm, oracle_price_data)?;
    let (short_reserves, long_reserves) =
        calculate_spread_reserves(&new_amm, oracle_price_data, None)?;

    let dir_reserves = if PositionDirection::Long == direction {
        long_reserves
    } else {
        short_reserves
    };

    Ok((
        dir_reserves.0,
        dir_reserves.1,
        new_amm.sqrt_k,
        new_amm.peg_multiplier,
    ))
}

/// Calculates what the amm reserves would be after swapping a quote or base asset amount.
pub fn calculate_amm_reserves_after_swap(
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

// pub fn calculate_market_open_bid_ask(
//     base_asset_reserve: u128,
//     min_base_asset_reserve: u128,
//     max_base_asset_reserve: u128,
//     step_size: Option<u64>,
// ) -> (i64, i64) {
//     // open orders
//     let mut open_asks: i64 = 0;
//     if min_base_asset_reserve < base_asset_reserve {
//         open_asks = (base_asset_reserve - min_base_asset_reserve) as i64 * -1;
//
//         if let Some(step_size) = step_size {
//             if open_asks / 2 < step_size as i64 {
//                 open_asks = 0;
//             }
//         }
//     } else {
//         open_asks = 0;
//     }
//
//     let mut open_bids: i64 = 0;
//     if max_base_asset_reserve > base_asset_reserve {
//         open_bids = (max_base_asset_reserve - base_asset_reserve) as i64;
//
//         if let Some(step_size) = step_size {
//             if open_bids / 2 < step_size as i64 {
//                 open_bids = 0;
//             }
//         }
//     } else {
//         open_bids = 0;
//     }
//
//     return (open_bids, open_asks);
// }

// pub fn  calculate_inventory_liquidity_ratio(
// 	baseAssetAmountWithAmm: BN,
// 	baseAssetReserve: BN,
// 	minBaseAssetReserve: BN,
// 	maxBaseAssetReserve: BN
// ) ->  BN {
//    calculate_inv
// 	// inventory skew
// 	const [openBids, openAsks] = calculateMarketOpenBidAsk(
// 		baseAssetReserve,
// 		minBaseAssetReserve,
// 		maxBaseAssetReserve
// 	);

// 	const minSideLiquidity = BN.min(openBids.abs(), openAsks.abs());

// 	const inventoryScaleBN = BN.min(
// 		baseAssetAmountWithAmm
// 			.mul(PERCENTAGE_PRECISION)
// 			.div(BN.max(minSideLiquidity, ONE))
// 			.abs(),
// 		PERCENTAGE_PRECISION
// 	);
// 	return inventoryScaleBN;
// }

// pub fn calculate_inventory_scale(
//     base_asset_amount_with_amm: i128,
//     base_asset_reserve: u128,
//     min_base_asset_reserve: u128,
//     max_base_asset_reserve: u128,
//     directional_spread: u64,
//     max_spread: u64,
// ) -> SdkResult<u128> {
//     if base_asset_amount_with_amm == 0 {
//         return 1;
//     }
//
//     let max_bid_ask_inventory_skew_factor = BID_ASK_SPREAD_PRECISION.mul(10);
//
//     let inventory_scale_bn = calculate_inventory_liquidity_ratio(
//         base_asset_amount_with_amm,
//         base_asset_reserve,
//         min_base_asset_reserve,
//         max_base_asset_reserve,
//     )?;
//
//     let inventory_scale_max_bn = std::cmp::max(
//         max_bid_ask_inventory_skew_factor,
//         max_spread
//             .mul(BID_ASK_SPREAD_PRECISION)
//             .div(std::cmp::max(directional_spread, 1)),
//     );
//
//     let inventory_scale_capped = std::cmp::min(
//         inventory_scale_max_bn,
//         BID_ASK_SPREAD_PRECISION.add(
//             inventory_scale_max_bn
//                 .mul(inventory_scale_bn)
//                 .div(PERCENTAGE_PRECISION),
//         ),
//     ) / BID_ASK_SPREAD_PRECISION;
//
//     return inventoryScaleCapped;
// }

pub fn calculate_vol_spread_bn(
    last_oracle_conf_pct: u128,
    reserve_price: u128,
    mark_std: u128,
    oracle_std: u128,
    long_intensity: u128,
    short_intensity: u128,
    volume_24h: u128,
) -> (u128, u128) {
    let market_avg_std_pct = mark_std
        .add(oracle_std)
        .mul(PERCENTAGE_PRECISION)
        .div(reserve_price)
        .div(2);
    let vol_spread = std::cmp::max(last_oracle_conf_pct, market_avg_std_pct.div(2));

    let clamp_min = PERCENTAGE_PRECISION.div(100);
    let clamp_max = PERCENTAGE_PRECISION.mul(16).div(10);

    let long_vol_spread_factor = clamp_bn(
        long_intensity
            .mul(PERCENTAGE_PRECISION)
            .div(std::cmp::max(1, volume_24h)),
        clamp_min,
        clamp_max,
    );
    let short_vol_spread_factor = clamp_bn(
        short_intensity
            .mul(PERCENTAGE_PRECISION)
            .div(std::cmp::max(1, volume_24h)),
        clamp_min,
        clamp_max,
    );

    let mut conf_component = last_oracle_conf_pct;

    if last_oracle_conf_pct <= PRICE_PRECISION.div(400) {
        conf_component = last_oracle_conf_pct.div(10);
    }

    let long_vol_spread = std::cmp::max(
        conf_component,
        vol_spread
            .mul(long_vol_spread_factor)
            .div(PERCENTAGE_PRECISION),
    );
    let short_vol_spread = std::cmp::max(
        conf_component,
        vol_spread
            .mul(short_vol_spread_factor)
            .div(PERCENTAGE_PRECISION),
    );

    (long_vol_spread, short_vol_spread)
}

// pub fn calculate_spread_bn(
//     base_spread: u128,
//     last_oracle_reserve_price_spread_pct: f64,
//     last_oracle_conf_pct: u128,
//     max_spread: f64,
//     quote_asset_reserve: u128,
//     terminal_quote_asset_reserve: u128,
//     peg_multiplier: u128,
//     base_asset_amount_with_amm: i128,
//     reserve_price: u128,
//     total_fee_minus_distributions: u128,
//     net_revenue_since_last_funding: u128,
//     base_asset_reserve: u128,
//     min_base_asset_reserve: u128,
//     max_base_asset_reserve: u128,
//     mark_std: u128,
//     oracle_std: u128,
//     long_intensity: u128,
//     short_intensity: u128,
//     volume_24h: u128,
//     return_terms: bool,
// ) -> (Option<SpreadTerms>, Option<(i128, i128)>) {
//     let mut spread_terms = SpreadTerms::default();
//
//     let (long_vol_spread, short_vol_spread) = calculate_vol_spread_bn(
//         last_oracle_conf_pct,
//         reserve_price,
//         mark_std,
//         oracle_std,
//         long_intensity,
//         short_intensity,
//         volume_24h,
//     );
//
//     spread_terms.long_vol_spread = long_vol_spread;
//     spread_terms.short_vol_spread = short_vol_spread;
//
//     let mut long_spread = std::cmp::max(base_spread / 2, long_vol_spread);
//     let mut short_spread = std::cmp::max(base_spread / 2, short_vol_spread);
//
//     if last_oracle_reserve_price_spread_pct > 0.0 {
//         short_spread = std::cmp::max(
//             short_spread,
//             last_oracle_reserve_price_spread_pct + long_vol_spread,
//         );
//     } else if last_oracle_reserve_price_spread_pct < 0.0 {
//         long_spread = std::cmp::max(
//             long_spread,
//             last_oracle_reserve_price_spread_pct + long_vol_spread,
//         );
//     }
//     spread_terms.long_spread_w_ps = long_spread;
//     spread_terms.short_spread_w_ps = short_spread;
//
//     let max_value = max_spread.max(last_oracle_reserve_price_spread_pct.abs());
//     let max_target_spread = max_value.floor();
//
//     // let inventory_spread_scale = calculate_in
//
//     (None, None)
// }

pub fn calculate_spread(
    amm: &AMM,
    oracle_price_data: Option<&OraclePriceData>,
    now: Option<i64>,
    reserve_price: Option<u64>,
) -> SdkResult<(u32, u32)> {
    let reserve_price = match reserve_price {
        Some(price) => price,
        None => calculate_price(
            amm.base_asset_reserve,
            amm.quote_asset_reserve,
            amm.peg_multiplier,
        )?,
    };

    let target_price = match oracle_price_data {
        Some(data) => data.price as u64,
        None => reserve_price,
    };
    let target_mark_spread_pct = reserve_price
        .sub(target_price)
        .mul(BID_ASK_SPREAD_PRECISION)
        .div(reserve_price);

    let now = match now {
        Some(time) => time,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    };

    let live_oracle_std = calculate_live_oracle_std(amm, oracle_price_data.unwrap(), now);
    let conf_interval_pct =
        get_new_oracle_conf_pct(amm, oracle_price_data.unwrap(), reserve_price, now);

    let spreads = drift::math::amm_spread::calculate_spread(
        amm.base_spread,
        target_mark_spread_pct as i64,
        conf_interval_pct,
        amm.max_spread,
        amm.quote_asset_reserve,
        amm.terminal_quote_asset_reserve,
        amm.peg_multiplier,
        amm.base_asset_amount_with_amm,
        reserve_price,
        amm.total_fee_minus_distributions,
        amm.net_revenue_since_last_funding,
        amm.base_asset_reserve,
        amm.min_base_asset_reserve,
        amm.max_base_asset_reserve,
        amm.mark_std,
        live_oracle_std as u64,
        amm.long_intensity_volume,
        amm.short_intensity_volume,
        amm.volume_24h,
    )?;

    Ok((spreads.0, spreads.1))
}

/// Return `bid_reserves`, `ask_reserves`
pub fn calculate_spread_reserves(
    amm: &AMM,
    oracle_price_data: &OraclePriceData,
    now: Option<i64>,
) -> SdkResult<((u128, u128), (u128, u128))> {
    // fn calculate_spread_reserve(
    //     spread: i128,
    //     direction: PositionDirection,
    //     amm: &AMM,
    // ) -> (u128, u128) {
    //     if spread == 0 {
    //         return (amm.base_asset_reserve, amm.quote_asset_reserve);
    //     }
    //     let mut spread_fraction = spread / 2;

    //     if spread_fraction == 0 {
    //         spread_fraction = if spread >= 0 { 1 } else { -1 };
    //     }

    //     let quote_asset_reserve_delta = amm
    //         .quote_asset_reserve
    //         .div((BID_ASK_SPREAD_PRECISION as i128 / spread_fraction) as u128);

    //     let quote_asset_reserve = if quote_asset_reserve_delta >= 0 {
    //         amm.quote_asset_reserve + quote_asset_reserve_delta
    //     } else {
    //         amm.quote_asset_reserve - quote_asset_reserve_delta
    //     };

    //     let base_asset_reserve = amm.sqrt_k.mul(amm.sqrt_k).div(quote_asset_reserve);

    //     (base_asset_reserve, quote_asset_reserve)
    // }

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
    }

    let (long_spread, short_spread) =
        calculate_spread(amm, Some(oracle_price_data), now, Some(reserve_price))?;

    let ask_reserves =
        drift::math::amm_spread::calculate_spread_reserves(amm, PositionDirection::Long)?;
    let bid_reserves =
        drift::math::amm_spread::calculate_spread_reserves(amm, PositionDirection::Short)?;

    Ok((bid_reserves, ask_reserves))
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

pub fn calculate_max_base_asset_amount_to_trade(
    amm: &AMM,
    limit_price: u64,
    direction: PositionDirection,
    oracle_price_data: &OraclePriceData,
    now: Option<i64>,
) -> SdkResult<(u128, PositionDirection)> {
    let invariant = amm.sqrt_k * amm.sqrt_k;

    let new_base_asset_reserve_squared =
        invariant * PRICE_PRECISION * amm.peg_multiplier / limit_price as u128 / PEG_PRECISION;

    let new_base_asset_reserve = square_root_u128(new_base_asset_reserve_squared);
    let (short_spread_reserves, long_spread_reserves) =
        calculate_spread_reserves(amm, oracle_price_data, now)?;

    let base_asset_reserve_before = if matches!(direction, PositionDirection::Long) {
        long_spread_reserves.0
    } else {
        short_spread_reserves.0
    };

    if new_base_asset_reserve > base_asset_reserve_before {
        return Ok((
            new_base_asset_reserve - base_asset_reserve_before,
            PositionDirection::Short,
        ));
    } else if new_base_asset_reserve < base_asset_reserve_before {
        return Ok((
            base_asset_reserve_before - new_base_asset_reserve,
            PositionDirection::Long,
        ));
    } else {
        log::info!("trade size too small");
        return Ok((0, PositionDirection::Long));
    }
}

pub fn calculate_max_base_asset_amount_fillable(
    amm: &AMM,
    order_direction: PositionDirection,
) -> SdkResult<u64> {
    let max_fill_size = amm.base_asset_reserve / amm.max_fill_reserve_fraction as u128;

    let max_base_asset_amount_on_side = if matches!(order_direction, PositionDirection::Long) {
        std::cmp::max(0, amm.base_asset_reserve - amm.min_base_asset_reserve)
    } else {
        std::cmp::max(0, amm.max_base_asset_reserve - amm.base_asset_reserve)
    };

    Ok(standardize_base_asset_amount(
        std::cmp::min(max_fill_size, max_base_asset_amount_on_side) as u64,
        amm.order_step_size,
    )?)
}
