use drift::{
    controller::position::PositionDirection,
    math::amm::calculate_price,
    state::{oracle::OraclePriceData, perp_market::PerpMarket},
};

use crate::types::SdkResult;

use super::amm::calculate_updated_amm_spread_reserves;

pub fn calculate_ask_price(market: &PerpMarket, oracle_price_data: &OraclePriceData) -> SdkResult<()> {
    let (base_asset_reserve, quote_asset_reserve, _sqrt_k, new_peg) =
        calculate_updated_amm_spread_reserves(
            &market.amm,
            PositionDirection::Long,
            oracle_price_data,
        );

    calculate_price(quote_asset_reserve, base_asset_reserve, new_peg)?
}
