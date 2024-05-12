use std::collections::HashMap;

use drift::state::{oracle::OraclePriceData, perp_market::PerpMarket};

use crate::types::SdkResult;

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
    oracle_price_data: OraclePriceData,
    num_orders: usize,
    now: u64,
    top_of_book_quote_amounts: Option<Vec<u64>>,
) -> SdkResult<()> {
    let mut num_base_orders = num_orders;
    if let Some(amounts) = top_of_book_quote_amounts {
        num_base_orders = num_orders - amounts.len();
        assert!(amounts.len() < num_orders);
    }



    Ok(())
}
