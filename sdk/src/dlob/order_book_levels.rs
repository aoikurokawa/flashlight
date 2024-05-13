use std::collections::HashMap;

use drift::{
    math::amm::calculate_market_open_bids_asks,
    state::{oracle::OraclePriceData, perp_market::PerpMarket},
};

use crate::{
    math::amm::{calculate_spread_reserves, calculate_updated_amm},
    types::SdkResult,
};

pub(crate) enum LiquiditySource {
    Serum,
    Vamm,
    Dlob,
    Phoenix,
}

pub(crate) struct L2Level {
    price: u64,
    size: u64,
    sources: HashMap<LiquiditySource, u64>,
}

pub(crate) struct L2OrderBook {
    pub(crate) asks: Vec<L2Level>,
    pub(crate) bids: Vec<L2Level>,
    pub(crate) slot: Option<u64>,
}

pub(crate) trait L2OrderBookGenerator {
    fn get_l2_asks() -> impl Iterator<Item = L2Level>;
    fn get_L2_bids() -> impl Iterator<Item = L2Level>;
}

pub(crate) fn get_vamm_l2_generator(
    market_account: PerpMarket,
    oracle_price_data: &OraclePriceData,
    num_orders: usize,
    now: Option<u128>,
    top_of_book_quote_amounts: Option<Vec<u64>>,
) -> SdkResult<()> {
    let mut num_base_orders = num_orders;
    if let Some(amounts) = top_of_book_quote_amounts {
        num_base_orders = num_orders - amounts.len();
        assert!(amounts.len() < num_orders);
    }

    let updated_amm = calculate_updated_amm(&market_account.amm, &oracle_price_data)?;

    let (mut open_bids, mut open_asks) = calculate_market_open_bids_asks(&updated_amm)?;

    let min_order_size = market_account.amm.min_order_size;
    if open_bids < min_order_size as i128 * 2 {
        open_bids = 0;
    }

    if open_asks.abs() < min_order_size as i128 * 2 {
        open_asks = 0;
    }

    let now = match now {
        Some(t) => t,
        None => {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                / 1000
        }
    };
    let (bid_reserves, ask_reserves) =
        calculate_spread_reserves(&updated_amm, oracle_price_data, Some(now))?;

    Ok(())
}
