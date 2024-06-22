use std::{
    collections::HashMap,
    ops::{Div, Mul, Sub},
};

use drift::{
    controller::amm::SwapDirection,
    math::{
        amm::{calculate_market_open_bids_asks, calculate_quote_asset_amount_swapped},
        constants::{BASE_PRECISION, QUOTE_PRECISION},
    },
    state::{
        oracle::OraclePriceData,
        perp_market::{PerpMarket, AMM},
        user::AssetType,
    },
};
use solana_sdk::pubkey::Pubkey;

use crate::{
    math::amm::{
        calculate_amm_reserves_after_swap, calculate_spread_reserves, calculate_updated_amm,
    },
    types::SdkResult,
};

use super::dlob_node::DLOBNode;

pub const DEFAULT_TOP_OF_BOOK_QUOTE_AMOUNTS: [u64; 4] = [
    500 * QUOTE_PRECISION as u64,
    1000 * QUOTE_PRECISION as u64,
    2000 * QUOTE_PRECISION as u64,
    5000 * QUOTE_PRECISION as u64,
];

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LiquiditySource {
    Serum,
    Vamm,
    Dlob,
    Phoenix,
}

pub struct L2Level {
    pub price: u128,
    pub size: i128,
    pub sources: HashMap<LiquiditySource, i128>,
}

impl L2Level {
    pub(crate) fn new(price: u128, size: i128, sources: HashMap<LiquiditySource, i128>) -> Self {
        Self {
            price,
            size,
            sources,
        }
    }
}

pub struct L2OrderBook {
    pub asks: Vec<L2Level>,
    pub bids: Vec<L2Level>,
    pub slot: u64,
}

pub trait L2OrderBookGenerator {
    fn get_l2_asks(&mut self) -> Box<dyn Iterator<Item = L2Level>>;
    fn get_l2_bids(&mut self) -> Box<dyn Iterator<Item = L2Level>>;
}

pub struct L3Level {
    pub price: u64,
    pub size: u64,
    pub maker: Pubkey,
    pub order_id: u32,
}

pub struct L3OrderBook {
    pub asks: Vec<L3Level>,
    pub bids: Vec<L3Level>,
    pub slot: u64,
}

struct L2Bids {
    num_bids: usize,
    num_orders: usize,
    bid_size: i128,
    top_of_book_quote_amounts: Option<Vec<u64>>,
    open_bids: i128,
    top_of_book_bid_size: i128,
    bid_amm: AMM,
    num_base_orders: usize,
}

impl Iterator for L2Bids {
    type Item = L2Level;

    fn next(&mut self) -> Option<Self::Item> {
        while self.num_bids < self.num_orders && self.bid_size < 0 {
            let mut quote_swapped = 0;
            let mut base_swapped = 0;
            let mut after_swap_quote_reserves = 0;
            let mut after_swap_base_reserves = 0;

            if let Some(ref top_of_book_quote_amounts) = self.top_of_book_quote_amounts {
                if self.num_bids < top_of_book_quote_amounts.len() {
                    let remaining_base_liquidity = self.open_bids - self.top_of_book_bid_size;
                    quote_swapped = top_of_book_quote_amounts[self.num_bids] as u128;
                    (after_swap_quote_reserves, after_swap_base_reserves) =
                        calculate_amm_reserves_after_swap(
                            &self.bid_amm,
                            AssetType::Quote,
                            quote_swapped as i128,
                            SwapDirection::Remove,
                        )
                        .ok()?;
                    base_swapped =
                        (self.bid_amm.base_asset_reserve - after_swap_base_reserves) as i128;

                    if remaining_base_liquidity < base_swapped {
                        base_swapped = remaining_base_liquidity;
                        (after_swap_quote_reserves, after_swap_base_reserves) =
                            calculate_amm_reserves_after_swap(
                                &self.bid_amm,
                                AssetType::Base,
                                base_swapped,
                                SwapDirection::Add,
                            )
                            .ok()?;
                        quote_swapped = calculate_quote_asset_amount_swapped(
                            self.bid_amm.quote_asset_reserve,
                            after_swap_quote_reserves,
                            SwapDirection::Add,
                            self.bid_amm.peg_multiplier,
                        )
                        .ok()?;
                    }

                    self.top_of_book_bid_size += base_swapped;
                    self.bid_size = self
                        .open_bids
                        .sub(self.top_of_book_bid_size)
                        .div(self.num_base_orders as i128);
                }
            } else {
                base_swapped = self.bid_size;
                (after_swap_quote_reserves, after_swap_base_reserves) =
                    calculate_amm_reserves_after_swap(
                        &self.bid_amm,
                        AssetType::Base,
                        base_swapped,
                        SwapDirection::Add,
                    )
                    .ok()?;

                quote_swapped = calculate_quote_asset_amount_swapped(
                    self.bid_amm.quote_asset_reserve,
                    after_swap_quote_reserves,
                    SwapDirection::Add,
                    self.bid_amm.peg_multiplier,
                )
                .ok()?;
            }

            let price = quote_swapped.mul(BASE_PRECISION).div(base_swapped as u128);

            self.bid_amm.base_asset_reserve = after_swap_base_reserves;
            self.bid_amm.quote_asset_reserve = after_swap_quote_reserves;

            self.num_bids += 1;

            let sources = HashMap::from([(LiquiditySource::Vamm, base_swapped)]);

            return Some(L2Level::new(price, base_swapped, sources));
        }

        None
    }
}

struct L2Asks {
    num_asks: usize,
    num_orders: usize,
    ask_size: i128,
    top_of_book_quote_amounts: Option<Vec<u64>>,
    open_asks: i128,
    top_of_book_ask_size: i128,
    ask_amm: AMM,
    num_base_orders: usize,
}

impl Iterator for L2Asks {
    type Item = L2Level;

    fn next(&mut self) -> Option<Self::Item> {
        while self.num_asks < self.num_orders && self.ask_size < 0 {
            let mut quote_swapped = 0;
            let mut base_swapped = 0;
            let mut after_swap_quote_reserves = 0;
            let mut after_swap_base_reserves = 0;

            if let Some(ref top_of_book_quote_amounts) = self.top_of_book_quote_amounts {
                if self.num_asks < top_of_book_quote_amounts.len() {
                    let remaining_base_liquidity =
                        self.open_asks.mul(-1).sub(self.top_of_book_ask_size);
                    quote_swapped = top_of_book_quote_amounts[self.num_asks] as u128;
                    (after_swap_quote_reserves, after_swap_base_reserves) =
                        calculate_amm_reserves_after_swap(
                            &self.ask_amm,
                            AssetType::Quote,
                            quote_swapped as i128,
                            SwapDirection::Add,
                        )
                        .ok()?;
                    base_swapped =
                        (self.ask_amm.base_asset_reserve - after_swap_base_reserves) as i128;

                    if base_swapped == 0 {
                        return None;
                    }

                    if remaining_base_liquidity < base_swapped {
                        base_swapped = remaining_base_liquidity;
                        (after_swap_quote_reserves, after_swap_base_reserves) =
                            calculate_amm_reserves_after_swap(
                                &self.ask_amm,
                                AssetType::Base,
                                base_swapped,
                                SwapDirection::Remove,
                            )
                            .ok()?;
                        quote_swapped = calculate_quote_asset_amount_swapped(
                            self.ask_amm.quote_asset_reserve,
                            after_swap_quote_reserves,
                            SwapDirection::Remove,
                            self.ask_amm.peg_multiplier,
                        )
                        .ok()?;
                    }

                    self.top_of_book_ask_size += base_swapped;
                    self.ask_size = self
                        .open_asks
                        .sub(self.top_of_book_ask_size)
                        .div(self.num_base_orders as i128);
                }
            } else {
                base_swapped = self.ask_size;
                (after_swap_quote_reserves, after_swap_base_reserves) =
                    calculate_amm_reserves_after_swap(
                        &self.ask_amm,
                        AssetType::Base,
                        base_swapped,
                        SwapDirection::Remove,
                    )
                    .ok()?;

                quote_swapped = calculate_quote_asset_amount_swapped(
                    self.ask_amm.quote_asset_reserve,
                    after_swap_quote_reserves,
                    SwapDirection::Remove,
                    self.ask_amm.peg_multiplier,
                )
                .ok()?;
            }

            let price = quote_swapped.mul(BASE_PRECISION).div(base_swapped as u128);

            self.ask_amm.base_asset_reserve = after_swap_base_reserves;
            self.ask_amm.quote_asset_reserve = after_swap_quote_reserves;

            self.num_asks += 1;

            let sources = HashMap::from([(LiquiditySource::Vamm, base_swapped)]);

            return Some(L2Level::new(price, base_swapped, sources));
        }

        None
    }
}

pub struct VammL2Generator {
    market_account: PerpMarket,
    oracle_price_data: OraclePriceData,
    num_orders: usize,
    now: i64,
    top_of_book_quote_amounts: Option<Vec<u64>>,

    bid_size: i128,

    bid_amm: AMM,

    open_bids: i128,

    num_base_orders: usize,

    ask_size: i128,

    open_asks: i128,

    ask_amm: AMM,
}

impl VammL2Generator {
    pub fn new(
        market_account: PerpMarket,
        oracle_price_data: &OraclePriceData,
        num_orders: usize,
        now: Option<i64>,
        top_of_book_quote_amounts: Option<Vec<u64>>,
    ) -> SdkResult<Self> {
        let mut num_base_orders = num_orders;
        if let Some(ref amounts) = top_of_book_quote_amounts {
            num_base_orders = num_orders - amounts.len();
            assert!(amounts.len() < num_orders);
        }

        let updated_amm = calculate_updated_amm(&market_account.amm, oracle_price_data)?;

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

        let bid_size = open_bids.div(num_base_orders as i128);
        let mut bid_amm = updated_amm;
        bid_amm.base_asset_reserve = bid_reserves.0;
        bid_amm.quote_asset_reserve = bid_reserves.1;

        bid_amm.base_asset_reserve = bid_reserves.0;
        bid_amm.quote_asset_reserve = bid_reserves.1;

        let ask_size = open_asks.abs().div(num_base_orders as i128);
        let mut ask_amm = updated_amm;
        ask_amm.base_asset_reserve = ask_reserves.0;
        ask_amm.quote_asset_reserve = ask_reserves.1;

        Ok(Self {
            market_account,
            oracle_price_data: *oracle_price_data,
            num_orders,
            now,
            top_of_book_quote_amounts,
            bid_size,
            bid_amm,
            open_bids,
            num_base_orders,
            ask_size,
            open_asks,
            ask_amm,
        })
    }

    pub fn get_l2_bids(&mut self) -> impl Iterator<Item = L2Level> {
        let num_bids = 0;
        let top_of_book_bid_size = 0;

        L2Bids {
            num_bids,
            num_orders: self.num_orders,
            bid_size: self.bid_size,
            top_of_book_quote_amounts: self.top_of_book_quote_amounts.clone(),
            open_bids: self.open_bids,
            top_of_book_bid_size,
            bid_amm: self.bid_amm,
            num_base_orders: self.num_base_orders,
        }
    }

    pub fn get_l2_asks(&mut self) -> impl Iterator<Item = L2Level> {
        let num_asks = 0;
        let top_of_book_ask_size = 0;

        L2Asks {
            num_asks,
            num_orders: self.num_orders,
            ask_size: self.ask_size,
            top_of_book_quote_amounts: self.top_of_book_quote_amounts.clone(),
            open_asks: self.open_asks,
            top_of_book_ask_size,
            ask_amm: self.ask_amm,
            num_base_orders: self.num_base_orders,
        }
    }
}

impl L2OrderBookGenerator for VammL2Generator {
    fn get_l2_asks(&mut self) -> Box<dyn Iterator<Item = L2Level>> {
        Box::new(self.get_l2_asks())
    }

    fn get_l2_bids(&mut self) -> Box<dyn Iterator<Item = L2Level>> {
        Box::new(self.get_l2_bids())
    }
}

pub fn get_l2_generator_from_dlob_nodes<I, T>(
    mut dlob_nodes: I,
    oracle_price_data: OraclePriceData,
    slot: u64,
) -> Box<dyn Iterator<Item = L2Level>>
where
    I: Iterator<Item = T> + 'static,
    T: DLOBNode,
{
    Box::new(std::iter::from_fn(move || {
        if let Some(dlob_node) = dlob_nodes.next() {
            let order = dlob_node.get_order();
            let size = order.base_asset_amount.sub(order.base_asset_amount_filled);

            let sources = HashMap::from([(LiquiditySource::Dlob, size as i128)]);
            Some(L2Level {
                price: dlob_node.get_price(&oracle_price_data, slot) as u128,
                size: size as i128,
                sources,
            })
        } else {
            None
        }
    }))
}

pub(crate) fn merge_l2_level_generators<I, F>(
    mut l2_level_generators: Vec<I>,
    compare: F,
) -> impl Iterator<Item = L2Level>
where
    I: Iterator<Item = L2Level>,
    F: Fn(&L2Level, &L2Level) -> bool,
{
    std::iter::from_fn(move || {
        let mut next = None;

        for generator in &mut l2_level_generators {
            if let Some(candidate) = generator.next() {
                match &next {
                    None => next = Some(candidate),
                    Some(best) => {
                        if compare(&candidate, best) {
                            next = Some(candidate);
                        }
                    }
                }
            }
        }

        next
    })
}

pub(crate) fn create_l2_levels(
    mut generator: impl Iterator<Item = L2Level>,
    depth: usize,
) -> Vec<L2Level> {
    let mut levels: Vec<L2Level> = Vec::new();

    if let Some(level) = generator.next() {
        let price = level.price;
        let size = level.size;
        let len = levels.len();

        if !levels.is_empty() && levels[len - 1].price == price {
            let current_level = &mut levels[len - 1];
            current_level.size += size;

            for (source, size) in level.sources.iter() {
                current_level
                    .sources
                    .entry(source.clone())
                    .and_modify(|entry| *entry += size)
                    .or_insert(*size);
            }
        } else if levels.len() == depth {
            return levels;
        } else {
            levels.push(level);
        }
    }

    levels
}
