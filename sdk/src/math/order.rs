use drift::{
    controller::position::PositionDirection,
    math::orders::standardize_base_asset_amount,
    state::{
        oracle::OraclePriceData,
        perp_market::{PerpMarket, AMM},
        user::{Order, OrderStatus, OrderTriggerCondition, OrderType},
    },
};

use crate::{
    math::auction::{get_auction_price, is_auction_complete},
    types::SdkResult,
};

use super::{
    amm::{
        calculate_max_base_asset_amount_fillable, calculate_max_base_asset_amount_to_trade,
        calculate_updated_amm,
    },
    auction::is_fallback_available_liquidity_source,
};

pub fn get_limit_price(
    order: &Order,
    oracle_price_data: &OraclePriceData,
    slot: u64,
    fallback_price: Option<u64>,
) -> Option<u64> {
    if has_auction_price(order, slot) {
        let price = get_auction_price(order, slot, oracle_price_data.price)
            .try_into()
            .unwrap();
        Some(price)
    } else if order.oracle_price_offset != 0 {
        let price = (oracle_price_data.price as i128 + order.oracle_price_offset as i128)
            .try_into()
            .unwrap();
        Some(price)
    } else if order.price == 0 {
        match fallback_price {
            Some(price) => Some(price),
            None => {
                dbg!("Order price is 0 and no fallback price provided: {}", order);
                None
            }
        }
    } else {
        Some(order.price)
    }
}

fn has_auction_price(order: &Order, slot: u64) -> bool {
    !is_auction_complete(order, slot)
        && (order.auction_start_price != 0 || order.auction_end_price != 0)
}

pub fn is_fillable_by_vamm(
    order: &Order,
    market: PerpMarket,
    oracle_price_data: &OraclePriceData,
    slot: u64,
    ts: i64,
    min_auction_duration: u8,
) -> SdkResult<bool> {
    let cond = is_fallback_available_liquidity_source(order, min_auction_duration, slot)
        && calculate_base_asset_amount_for_amm_to_fulfill(order, &market, oracle_price_data, slot)?
            >= market.amm.min_order_size
        || is_order_expired(order, ts, None, None);

    Ok(cond)
}

fn calculate_base_asset_amount_for_amm_to_fulfill(
    order: &Order,
    market: &PerpMarket,
    oracle_price_data: &OraclePriceData,
    slot: u64,
) -> SdkResult<u64> {
    if must_be_triggered(order) && !is_triggered(order) {}

    let limit_price = get_limit_price(order, oracle_price_data, slot, None);

    let updated_amm = calculate_updated_amm(&market.amm, oracle_price_data)?;
    let base_asset_amount = match limit_price {
        Some(limit_price) => calculate_base_asset_amount_to_fill_up_to_limit_price(
            order,
            &updated_amm,
            limit_price,
            oracle_price_data,
        )?,
        None => order.base_asset_amount - order.base_asset_amount_filled,
    };

    let max_base_asset_amount =
        calculate_max_base_asset_amount_fillable(&updated_amm, order.direction)?;

    Ok(std::cmp::min(max_base_asset_amount, base_asset_amount))
}

fn calculate_base_asset_amount_to_fill_up_to_limit_price(
    order: &Order,
    amm: &AMM,
    limit_price: u64,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<u64> {
    let adjusted_limit_price = if matches!(order.direction, PositionDirection::Long) {
        limit_price - amm.order_tick_size
    } else {
        limit_price + amm.order_tick_size
    };

    let (max_amount_to_trade, direction) = calculate_max_base_asset_amount_to_trade(
        amm,
        adjusted_limit_price,
        order.direction,
        oracle_price_data,
        None,
    )?;

    let base_asset_amount =
        standardize_base_asset_amount(max_amount_to_trade as u64, amm.order_step_size)?;

    // check that directions are the same
    if direction != order.direction {
        return Ok(0);
    }

    let base_asset_amount_unfilled = order.base_asset_amount - order.base_asset_amount_filled;

    if base_asset_amount > base_asset_amount_unfilled {
        Ok(base_asset_amount_unfilled)
    } else {
        Ok(base_asset_amount)
    }
}

pub fn is_order_expired(
    order: &Order,
    ts: i64,
    enforce_buffer: Option<bool>,
    buffer_seconds: Option<i64>,
) -> bool {
    let enforce_buffer = enforce_buffer.unwrap_or(false);
    let buffer_seconds = buffer_seconds.unwrap_or(15);

    if must_be_triggered(order) || OrderStatus::Open != order.status || order.max_ts == 0 {
        return false;
    }

    let max_ts = if enforce_buffer && order.is_limit_order() {
        order.max_ts + buffer_seconds
    } else {
        order.max_ts
    };

    ts > max_ts
}

pub fn must_be_triggered(order: &Order) -> bool {
    matches!(
        order.order_type,
        OrderType::TriggerMarket | OrderType::TriggerLimit
    )
}

pub fn is_triggered(order: &Order) -> bool {
    matches!(
        order.trigger_condition,
        OrderTriggerCondition::TriggeredAbove | OrderTriggerCondition::TriggeredBelow
    )
}

pub fn is_resting_limit_order(order: &Order, slot: u64) -> bool {
    if !order.is_limit_order() {
        return false;
    }

    if order.order_type == OrderType::TriggerLimit {
        return match order.direction {
            PositionDirection::Long if order.trigger_price < order.price => {
                return false;
            }
            PositionDirection::Short if order.trigger_price > order.price => {
                return false;
            }
            _ => is_auction_complete(order, slot),
        };
    };

    order.post_only || is_auction_complete(order, slot)
}
