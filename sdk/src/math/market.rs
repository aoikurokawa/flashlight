use drift::{
    controller::position::PositionDirection,
    math::amm::calculate_price,
    state::{oracle::OraclePriceData, perp_market::PerpMarket},
};

use crate::types::SdkResult;

use super::amm::calculate_updated_amm_spread_reserves;

/// Calculates market bid price
pub fn calculate_bid_price(
    market: &PerpMarket,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<u64> {
    let (base_asset_reserve, quote_asset_reserve, _sqrt_k, new_peg) =
        calculate_updated_amm_spread_reserves(
            &market.amm,
            PositionDirection::Short,
            oracle_price_data,
        )?;

    let price = calculate_price(quote_asset_reserve, base_asset_reserve, new_peg)?;

    Ok(price)
}

pub fn calculate_ask_price(
    market: &PerpMarket,
    oracle_price_data: &OraclePriceData,
) -> SdkResult<u64> {
    let (base_asset_reserve, quote_asset_reserve, _sqrt_k, new_peg) =
        calculate_updated_amm_spread_reserves(
            &market.amm,
            PositionDirection::Long,
            oracle_price_data,
        )?;

    let price = calculate_price(quote_asset_reserve, base_asset_reserve, new_peg)?;

    Ok(price)
}
