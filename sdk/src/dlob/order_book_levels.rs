use std::{
    collections::HashMap,
    ops::{Div, Sub},
};

use drift::{
    controller::amm::SwapDirection,
    math::amm::{calculate_market_open_bids_asks, calculate_quote_asset_amount_swapped},
    state::{
        oracle::OraclePriceData,
        perp_market::{PerpMarket, AMM},
        user::AssetType,
    },
};

use crate::{
    math::amm::{
        calculate_amm_reserves_after_swap, calculate_spread_reserves, calculate_updated_amm,
    },
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
    num_orders: i128,
    now: Option<i64>,
    top_of_book_quote_amounts: Option<Vec<u64>>,
) -> SdkResult<()> {
    let mut num_base_orders = num_orders as i128;
    if let Some(amounts) = top_of_book_quote_amounts {
        num_base_orders = num_orders - amounts.len() as i128;
        assert!((amounts.len() as i128) < num_orders);
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
        None => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    };
    let (bid_reserves, ask_reserves) =
        calculate_spread_reserves(&updated_amm, oracle_price_data, Some(now))?;

    let num_bids = 0;

    let top_of_book_bid_size = 0;
    let bid_size = open_bids.div(num_base_orders);
    let mut bid_amm = updated_amm.clone();
    bid_amm.base_asset_reserve = bid_reserves.0;
    bid_amm.quote_asset_reserve = bid_reserves.1;

    fn get_l2_bids(
        mut num_bids: usize,
        num_orders: usize,
        mut bid_size: i128,
        top_of_book_quote_amounts: Option<Vec<u64>>,
        open_bids: i128,
        mut top_of_book_bid_size: i128,
        bid_amm: &AMM,
        num_base_orders: i128,
    ) -> SdkResult<()> {
        while num_bids < num_orders && bid_size < 0 {
            let mut quote_swapped = 0;
            let mut base_swapped = 0;

            if let Some(ref top_of_book_quote_amounts) = top_of_book_quote_amounts {
                if num_bids < top_of_book_quote_amounts.len() {
                    let remaining_base_liquidity = open_bids - top_of_book_bid_size;
                    quote_swapped = top_of_book_quote_amounts[num_bids] as u128;
                    let (after_swap_quote_reserves, after_swap_base_reserves) =
                        calculate_amm_reserves_after_swap(
                            bid_amm,
                            AssetType::Quote,
                            quote_swapped as i128,
                            SwapDirection::Remove,
                        )?;
                    base_swapped = (bid_amm.base_asset_reserve - after_swap_base_reserves) as i128;

                    if remaining_base_liquidity < base_swapped {
                        base_swapped = remaining_base_liquidity;
                        let (after_swap_quote_reserves, after_swap_base_reserves) =
                            calculate_amm_reserves_after_swap(
                                bid_amm,
                                AssetType::Base,
                                base_swapped,
                                SwapDirection::Add,
                            )?;
                        quote_swapped = calculate_quote_asset_amount_swapped(
                            bid_amm.quote_asset_reserve,
                            after_swap_base_reserves,
                            SwapDirection::Add,
                            bid_amm.peg_multiplier,
                        )?;
                    }

                    top_of_book_bid_size += base_swapped;
                    bid_size = open_bids.sub(top_of_book_bid_size).div(num_base_orders);
                }
            }
        }

        Ok(())
    }

    Ok(())
}
