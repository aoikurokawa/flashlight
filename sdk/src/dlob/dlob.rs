#![allow(clippy::module_inception)]

use dashmap::DashSet;
use drift::controller::position::PositionDirection;
use drift::state::oracle::OraclePriceData;
use drift::state::perp_market::PerpMarket;
use drift::state::spot_market::SpotMarket;
use drift::state::state::{ExchangeStatus, State};
use drift::state::user::{MarketType, Order, OrderStatus, OrderTriggerCondition, OrderType};
use rayon::prelude::*;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;
use std::collections::{BinaryHeap, HashMap};
use std::ops::Sub;
use std::str::FromStr;
use std::sync::Arc;

use crate::dlob::dlob_node::{
    create_node, get_order_signature, DLOBNode, DirectionalNode, Node, NodeType,
};
use crate::dlob::market::{get_node_subtype_and_type, Exchange, OpenOrders, SubType};
use crate::dlob::order_book_levels::{create_l2_levels, merge_l2_level_generators};
use crate::event_emitter::Event;
use crate::math::auction::is_fallback_available_liquidity_source;
use crate::math::exchange_status::fill_paused;
use crate::math::order::{
    get_limit_price, is_order_expired, is_resting_limit_order, is_triggered, must_be_triggered,
};
use crate::types::SdkResult;
use crate::usermap::UserMap;
use crate::utils::market_type_to_string;

use super::order_book_levels::{
    get_l2_generator_from_dlob_nodes, L2OrderBook, L2OrderBookGenerator, L3Level, L3OrderBook,
};
use super::order_list::Orderlist;

#[derive(Debug, Clone)]
pub struct NodeToFill {
    node: Node,
    maker_nodes: Vec<Node>,
}

impl NodeToFill {
    pub fn new(node: Node, maker_nodes: Vec<Node>) -> Self {
        Self { node, maker_nodes }
    }

    pub fn get_node(&self) -> Node {
        self.node
    }

    pub fn get_maker_nodes(&self) -> &[Node] {
        &self.maker_nodes
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MarketAccount {
    PerpMarket(PerpMarket),
    SpotMarket(SpotMarket),
}

#[derive(Clone)]
pub struct DLOB {
    exchange: Exchange,
    open_orders: OpenOrders,
    initialized: bool,
    _max_slot_for_resting_limit_orders: Arc<u64>,
}

impl DLOB {
    pub fn new() -> DLOB {
        let exchange = Exchange::new();

        let open_orders = OpenOrders::new();
        open_orders.insert("perp".to_string(), DashSet::new());
        open_orders.insert("spot".to_string(), DashSet::new());

        DLOB {
            exchange,
            open_orders,
            initialized: true,
            _max_slot_for_resting_limit_orders: Arc::new(0),
        }
    }

    pub fn clear(&mut self) {
        self.exchange.clear();
        self.open_orders.clear();
        self.initialized = false;
        self._max_slot_for_resting_limit_orders = Arc::new(0);
    }

    /// Initializes a new DLOB instance
    pub fn build_from_usermap(&mut self, usermap: &UserMap, slot: u64) {
        self.clear();
        usermap.usermap.iter().par_bridge().for_each(|user_ref| {
            let user = user_ref.value();
            let user_key = user_ref.key();
            let user_pubkey = Pubkey::from_str(user_key).expect("Valid pubkey");
            for order in user.orders.iter() {
                if order.status == OrderStatus::Init {
                    continue;
                }
                self.insert_order(order, user_pubkey, slot);
            }
        });
        self.initialized = true;
    }

    pub fn size(&self) -> (usize, usize) {
        (self.exchange.perp_size(), self.exchange.spot_size())
    }

    /// for debugging
    pub fn print_all_spot_orders(&self) {
        for market in self.exchange.spot.iter() {
            println!("market index: {}", market.key());
            market.value().print_all_orders();
        }
    }

    pub fn insert_order(&self, order: &Order, user_account: Pubkey, slot: u64) {
        let market_type = market_type_to_string(&order.market_type);
        let market_index = order.market_index;

        let (subtype, node_type) = get_node_subtype_and_type(order, slot);
        let node = create_node(node_type, *order, user_account);

        self.exchange
            .add_market_indempotent(&market_type, market_index);

        let mut market = match order.market_type {
            MarketType::Perp => self.exchange.perp.get_mut(&market_index).expect("market"),
            MarketType::Spot => self.exchange.spot.get_mut(&market_index).expect("market"),
        };

        let order_list = market.get_order_list_for_node_insert(node_type);

        match subtype {
            SubType::Bid => order_list.insert_bid(node),
            SubType::Ask => order_list.insert_ask(node),
            sub_type => {
                log::error!("Subtype: {sub_type:?}");
            }
        }
    }

    pub fn get_order(&self, order_id: u32, user_account: Pubkey) -> Option<Order> {
        let order_signature = get_order_signature(order_id, user_account);
        for order_list in self.exchange.get_order_lists() {
            if let Some(node) = order_list.get_node(&order_signature) {
                return Some(*node.get_order());
            }
        }

        None
    }

    pub fn get_list_for_order(&self, order: &Order, slot: u64) -> Option<Orderlist> {
        let is_inactive_trigger_order = must_be_triggered(order) && !is_triggered(order);

        let node_type = if is_inactive_trigger_order {
            NodeType::Trigger
        } else if matches!(
            order.order_type,
            OrderType::Market | OrderType::TriggerMarket | OrderType::Oracle
        ) {
            NodeType::Market
        } else if order.oracle_price_offset != 0 {
            NodeType::FloatingLimit
        } else {
            let is_resting = is_resting_limit_order(order, slot);
            if is_resting {
                NodeType::RestingLimit
            } else {
                NodeType::TakingLimit
            }
        };

        let sub_type = if is_inactive_trigger_order {
            if matches!(order.trigger_condition, OrderTriggerCondition::Above) {
                SubType::Above
            } else {
                SubType::Below
            }
        } else {
            if matches!(order.direction, PositionDirection::Long) {
                SubType::Bid
            } else {
                SubType::Ask
            }
        };

        match order.market_type {
            MarketType::Perp => {
                if let Some(market) = self.exchange.perp.get(&order.market_index) {
                    let order_list = market.get_order_list_for_node_type(node_type);
                    let nodes = match sub_type {
                        SubType::Ask | SubType::Below => order_list,
                        SubType::Bid | SubType::Above => order_list,
                    };
                    return Some(nodes);
                }
            }
            MarketType::Spot => {
                if let Some(market) = self.exchange.spot.get(&order.market_index) {
                    let order_list = market.get_order_list_for_node_type(node_type);
                    let nodes = match sub_type {
                        SubType::Ask | SubType::Below => order_list,
                        SubType::Bid | SubType::Above => order_list,
                    };
                    return Some(nodes);
                }
            }
        }

        None
    }

    pub fn find_nodes_to_fill(
        &mut self,
        market_index: u16,
        fallback_bid: u64,
        fallback_ask: u64,
        slot: u64,
        ts: i64,
        market_type: MarketType,
        oracle_price_data: &OraclePriceData,
        state_account: &State,
        market_account: &MarketAccount,
    ) -> SdkResult<Vec<NodeToFill>> {
        if fill_paused(state_account, market_account) {
            return Ok(vec![]);
        }

        let is_amm_paused = state_account.amm_paused()?;

        let min_auction_duration = if MarketType::Perp == market_type {
            state_account.min_perp_auction_duration as u8
        } else {
            0
        };

        let (maker_rebate_numerator, maker_rebate_denominator) =
            self.get_maker_rebate(market_type, &state_account, market_account);

        let resting_limit_order_nodes_to_fill = self.find_resting_limit_order_nodes_to_fill(
            market_index,
            slot,
            market_type,
            oracle_price_data,
            is_amm_paused,
            min_auction_duration,
            maker_rebate_numerator as u64,
            maker_rebate_denominator as u64,
            Some(fallback_ask),
            Some(fallback_bid),
        );

        let taking_order_nodes_to_fill = self.find_taking_nodes_to_fill(
            market_index,
            slot,
            &market_type,
            oracle_price_data,
            is_amm_paused,
            min_auction_duration,
            Some(fallback_ask),
            Some(fallback_bid),
        )?;

        // get expired market nodes
        let expired_nodes_to_fill = self.find_expired_nodes_to_fill(market_index, ts, market_type);

        let mut nodes_to_fill = self.merge_nodes_to_fill(
            &resting_limit_order_nodes_to_fill,
            &taking_order_nodes_to_fill,
        );
        nodes_to_fill.extend(expired_nodes_to_fill);

        Ok(nodes_to_fill)
    }

    fn get_maker_rebate(
        &self,
        market_type: MarketType,
        state_account: &State,
        market_account: &MarketAccount,
    ) -> (u32, u32) {
        let (mut marker_rebate_numerator, maker_rebate_denominator) =
            if MarketType::Perp == market_type {
                (
                    state_account.perp_fee_structure.fee_tiers[0].maker_rebate_numerator,
                    state_account.perp_fee_structure.fee_tiers[0].maker_rebate_denominator,
                )
            } else {
                (
                    state_account.spot_fee_structure.fee_tiers[0].maker_rebate_numerator,
                    state_account.spot_fee_structure.fee_tiers[0].maker_rebate_denominator,
                )
            };

        let fee_adjustment = if let MarketAccount::PerpMarket(perp) = market_account {
            perp.fee_adjustment | 0
        } else {
            0
        };
        if fee_adjustment != 0 {
            marker_rebate_numerator += (maker_rebate_denominator * fee_adjustment as u32) / 100;
        }

        (marker_rebate_numerator, maker_rebate_denominator)
    }

    fn merge_nodes_to_fill(
        &self,
        resting_limit_order_nodes_to_fill: &[NodeToFill],
        taking_order_nodes_to_fill: &[NodeToFill],
    ) -> Vec<NodeToFill> {
        let mut merged_nodes_to_fill = HashMap::new();

        let mut merged_nodes_to_fill_helper = |nodes_to_fill_array: &[NodeToFill]| {
            for node_to_fill in nodes_to_fill_array {
                let node_signature = get_order_signature(
                    node_to_fill.node.get_order().order_id,
                    node_to_fill.node.get_user_account(),
                );

                if !merged_nodes_to_fill.contains_key(&node_signature) {
                    merged_nodes_to_fill.insert(
                        node_signature.clone(),
                        NodeToFill {
                            node: node_to_fill.node,
                            maker_nodes: vec![],
                        },
                    );
                }

                if !node_to_fill.maker_nodes.is_empty() {
                    if let Some(node) = merged_nodes_to_fill.get_mut(&node_signature) {
                        node.maker_nodes.extend(node_to_fill.maker_nodes.clone());
                    }
                }
            }
        };

        merged_nodes_to_fill_helper(resting_limit_order_nodes_to_fill);
        merged_nodes_to_fill_helper(taking_order_nodes_to_fill);

        let array = merged_nodes_to_fill
            .values()
            .into_iter()
            .map(|val| NodeToFill {
                node: val.node,
                maker_nodes: val.maker_nodes.to_vec(),
            })
            .collect();

        array
    }

    pub fn find_resting_limit_order_nodes_to_fill(
        &mut self,
        market_index: u16,
        slot: u64,
        market_type: MarketType,
        oracle_price_data: &OraclePriceData,
        is_amm_paused: bool,
        min_auction_duration: u8,
        maker_rebate_numerator: u64,
        maker_rebate_denominator: u64,
        fallback_ask: Option<u64>,
        fallback_bid: Option<u64>,
    ) -> Vec<NodeToFill> {
        let mut nodes_to_fill = Vec::new();

        let crossing_nodes = self.find_crossing_resting_limit_orders(
            market_index,
            slot,
            &market_type,
            oracle_price_data,
        );

        nodes_to_fill.extend(crossing_nodes);

        if let Some(fallback_bid) = fallback_bid {
            if !is_amm_paused {
                let ask_generator = self.get_resting_limit_asks(
                    slot,
                    &market_type,
                    market_index,
                    oracle_price_data,
                );

                let fallback_bid_with_buffer = fallback_bid
                    - (fallback_bid * maker_rebate_numerator / maker_rebate_denominator);

                let asks_crossing_fallback = self.find_nodes_crossing_fallback_liquidity(
                    &market_type,
                    slot,
                    oracle_price_data,
                    &ask_generator,
                    |ask_price| ask_price <= Some(fallback_bid_with_buffer),
                    min_auction_duration,
                );

                for ask_crossing_fallback in asks_crossing_fallback {
                    nodes_to_fill.push(ask_crossing_fallback);
                }
            }
        }

        if let Some(fallback_ask) = fallback_ask {
            if !is_amm_paused {
                let bid_generator = self.get_resting_limit_bids(
                    slot,
                    &market_type,
                    market_index,
                    oracle_price_data,
                );

                let fallback_ask_with_buffer = fallback_ask
                    - (fallback_ask * maker_rebate_numerator / maker_rebate_denominator);

                let bids_crossing_fallback = self.find_nodes_crossing_fallback_liquidity(
                    &market_type,
                    slot,
                    oracle_price_data,
                    &bid_generator,
                    |bid_price| bid_price <= Some(fallback_ask_with_buffer),
                    min_auction_duration,
                );

                for bid_crossing_fallback in bids_crossing_fallback {
                    nodes_to_fill.push(bid_crossing_fallback);
                }
            }
        }

        nodes_to_fill
    }

    pub fn find_taking_nodes_to_fill(
        &mut self,
        market_index: u16,
        slot: u64,
        market_type: &MarketType,
        oracle_price_data: &OraclePriceData,
        is_amm_paused: bool,
        min_auction_duration: u8,
        fallback_ask: Option<u64>,
        fallback_bid: Option<u64>,
    ) -> SdkResult<Vec<NodeToFill>> {
        let mut nodes_to_fill = Vec::new();

        let mut taking_order_generator = self.get_taking_asks(
            market_index,
            market_type,
            slot,
            oracle_price_data,
            NodeType::TakingLimit,
        )?;

        let taking_asks_crossing_bids = self.find_taking_nodes_crossing_maker_nodes(
            market_index,
            slot,
            market_type,
            oracle_price_data,
            taking_order_generator,
            |taker_price, maker_price| {
                if &MarketType::Spot == market_type {
                    if taker_price.is_none() {
                        return false;
                    }

                    if fallback_bid.is_some() && maker_price < fallback_bid.unwrap() {
                        return false;
                    }

                    return taker_price.is_none() || taker_price.unwrap() <= maker_price;
                }

                false
            },
        );

        nodes_to_fill.extend(taking_asks_crossing_bids);

        if let Some(fallback_bid) = fallback_bid {
            taking_order_generator = self.get_taking_asks(
                market_index,
                market_type,
                slot,
                oracle_price_data,
                NodeType::TakingLimit,
            )?;

            let taking_asks_crossing_fallback = self.find_nodes_crossing_fallback_liquidity(
                market_type,
                slot,
                oracle_price_data,
                &taking_order_generator,
                |taker_price| taker_price.is_none() || taker_price.unwrap() <= fallback_bid,
                min_auction_duration,
            );

            nodes_to_fill.extend(taking_asks_crossing_fallback);
        }

        taking_order_generator = self.get_taking_bids(
            market_index,
            market_type,
            slot,
            oracle_price_data,
            NodeType::TakingLimit,
        )?;

        let taking_bids_to_fill = self.find_taking_nodes_crossing_maker_nodes(
            market_index,
            slot,
            market_type,
            oracle_price_data,
            taking_order_generator,
            |taker_price, maker_price| {
                if &MarketType::Spot == market_type {
                    if taker_price.is_none() {
                        return false;
                    }

                    if fallback_bid.is_some() && maker_price < fallback_bid.unwrap() {
                        return false;
                    }

                    return taker_price.is_none() || taker_price.unwrap() >= maker_price;
                }

                false
            },
        );

        nodes_to_fill.extend(taking_bids_to_fill);

        if let Some(fallback_ask) = fallback_ask {
            if !is_amm_paused {
                taking_order_generator = self.get_taking_bids(
                    market_index,
                    market_type,
                    slot,
                    oracle_price_data,
                    NodeType::TakingLimit,
                )?;
                let taking_bids_crossing_fallback = self.find_nodes_crossing_fallback_liquidity(
                    market_type,
                    slot,
                    oracle_price_data,
                    &taking_order_generator,
                    |taker_price| taker_price.is_none() || taker_price.unwrap() >= fallback_ask,
                    min_auction_duration,
                );

                nodes_to_fill.extend(taking_bids_crossing_fallback);
            }
        }

        Ok(nodes_to_fill)
    }

    pub fn find_taking_nodes_crossing_maker_nodes<F>(
        &mut self,
        market_index: u16,
        slot: u64,
        market_type: &MarketType,
        oracle_price_data: &OraclePriceData,
        taker_node_generator: Vec<Node>,
        // maker_node_generator_fn: FA,
        does_cross: F,
    ) -> Vec<NodeToFill>
    where
        F: Fn(Option<u64>, u64) -> bool,
    {
        let mut nodes_to_fill = Vec::new();

        for taker_node in taker_node_generator {
            // let maker_node_generator =
            //     maker_node_generator_fn(market_index, slot, market_type, oracle_price_data);
            // FIXME: use generator
            let maker_node_generator =
                self.get_resting_limit_bids(slot, market_type, market_index, oracle_price_data);

            for maker_node in maker_node_generator {
                // Can't match orders from the same user
                let same_user = taker_node.get_user_account() == maker_node.get_user_account();
                if same_user {
                    continue;
                }

                let maker_price = maker_node.get_price(oracle_price_data, slot);
                let taker_price = taker_node.get_price(oracle_price_data, slot);

                let order_cross = does_cross(Some(taker_price), maker_price);
                if !order_cross {
                    // market orders aren't sorted by price, they are sorted by time, so we need to
                    // traverse through all of em
                    break;
                }

                nodes_to_fill.push(NodeToFill::new(taker_node, vec![maker_node]));

                let maker_order = maker_node.get_order();
                let taker_order = taker_node.get_order();

                let maker_base_remaining =
                    maker_order.base_asset_amount - maker_order.base_asset_amount_filled;
                let taker_base_remaining =
                    taker_order.base_asset_amount - taker_order.base_asset_amount_filled;

                let base_filled = std::cmp::min(maker_base_remaining, taker_base_remaining);

                let mut new_maker_order = maker_order.clone();
                new_maker_order.base_asset_amount_filled =
                    maker_order.base_asset_amount_filled + base_filled;
                if let Some(mut orders) = self.get_list_for_order(&new_maker_order, slot) {
                    let (_sub_type, node_type) = get_node_subtype_and_type(&new_maker_order, slot);
                    let order_node =
                        create_node(node_type, new_maker_order, maker_node.get_user_account());
                    orders.update_bid(order_node);
                }

                let mut new_taker_order = taker_order.clone();
                new_taker_order.base_asset_amount_filled =
                    taker_order.base_asset_amount_filled + base_filled;
                if let Some(mut orders) = self.get_list_for_order(&new_taker_order, slot) {
                    let (_sub_type, node_type) = get_node_subtype_and_type(&new_taker_order, slot);
                    let order_node =
                        create_node(node_type, new_taker_order, maker_node.get_user_account());
                    orders.update_bid(order_node);
                }

                if new_taker_order.base_asset_amount_filled == taker_order.base_asset_amount {
                    break;
                }
            }
        }

        nodes_to_fill
    }

    /// Return `node`, `maker_nodes`
    pub fn find_nodes_crossing_fallback_liquidity<F>(
        &mut self,
        market_type: &MarketType,
        slot: u64,
        oracle_price_data: &OraclePriceData,
        node_generator: &[Node],
        does_cross: F,
        min_auction_duration: u8,
    ) -> Vec<NodeToFill>
    where
        F: Fn(Option<u64>) -> bool,
    {
        let mut nodes_to_fill = Vec::new();

        for node in node_generator {
            let order = node.get_order();
            if &MarketType::Spot == market_type && order.post_only {
                continue;
            }

            let node_price = get_limit_price(order, oracle_price_data, slot, None);

            // order crosses if there is no limit price or it crosses fallback price
            let crosses = does_cross(node_price);

            let fallback_available = &MarketType::Spot == market_type
                || is_fallback_available_liquidity_source(order, min_auction_duration, slot);

            if crosses && fallback_available {
                nodes_to_fill.push(NodeToFill::new(*node, vec![]));
            }
        }

        nodes_to_fill
    }

    pub fn find_expired_nodes_to_fill(
        &self,
        market_index: u16,
        ts: i64,
        market_type: MarketType,
    ) -> Vec<NodeToFill> {
        let mut nodes_to_fill = Vec::new();

        let markets = match market_type {
            MarketType::Perp => &self.exchange.perp,
            MarketType::Spot => &self.exchange.spot,
        };

        // All bids/asks that can expire
        // dont try to expire limit orders with tif as its inefficient use of blockspace
        let mut bid_generators = Vec::new();
        if let Some(market) = markets.get(&market_index) {
            bid_generators.extend(market.taking_limit_orders.bids.clone());
            bid_generators.extend(market.resting_limit_orders.bids.clone());
            bid_generators.extend(market.floating_limit_orders.bids.clone());
            bid_generators.extend(market.market_orders.bids.clone());
        }

        let mut ask_generators = Vec::new();
        if let Some(market) = markets.get(&market_index) {
            ask_generators.extend(market.taking_limit_orders.asks.clone());
            ask_generators.extend(market.resting_limit_orders.asks.clone());
            ask_generators.extend(market.floating_limit_orders.asks.clone());
            ask_generators.extend(market.market_orders.asks.clone());
        }

        for bid in bid_generators {
            let order = bid.node.get_order();

            if is_order_expired(order, ts, Some(true), Some(25)) {
                nodes_to_fill.push(NodeToFill::new(bid.node, vec![]));
            }
        }

        for ask in ask_generators {
            let order = ask.node.get_order();

            if is_order_expired(order, ts, Some(true), Some(25)) {
                nodes_to_fill.push(NodeToFill::new(ask.node, vec![]));
            }
        }

        nodes_to_fill
    }

    fn update_resting_limit_orders_for_market_type(&mut self, slot: u64, market_type: MarketType) {
        let mut new_taking_asks: BinaryHeap<DirectionalNode> = BinaryHeap::new();
        let mut new_taking_bids: BinaryHeap<DirectionalNode> = BinaryHeap::new();

        let market = match market_type {
            MarketType::Perp => &self.exchange.perp,
            MarketType::Spot => &self.exchange.spot,
        };

        for mut market_ref in market.iter_mut() {
            let market = market_ref.value_mut();

            for directional_node in market.taking_limit_orders.bids.iter() {
                if is_resting_limit_order(directional_node.node.get_order(), slot) {
                    market
                        .resting_limit_orders
                        .insert_bid(directional_node.node)
                } else {
                    new_taking_bids.push(*directional_node)
                }
            }

            for directional_node in market.taking_limit_orders.asks.iter() {
                if is_resting_limit_order(directional_node.node.get_order(), slot) {
                    market
                        .resting_limit_orders
                        .insert_ask(directional_node.node);
                } else {
                    new_taking_asks.push(*directional_node);
                }
            }

            market.taking_limit_orders.bids = new_taking_bids.clone();
            market.taking_limit_orders.asks = new_taking_asks.clone();
        }
    }

    pub fn update_resting_limit_orders(&mut self, slot: u64) {
        if slot <= *self._max_slot_for_resting_limit_orders {
            return;
        }

        self._max_slot_for_resting_limit_orders = Arc::new(slot);

        self.update_resting_limit_orders_for_market_type(slot, MarketType::Perp);
        self.update_resting_limit_orders_for_market_type(slot, MarketType::Spot);
    }

    fn get_taking_bids(
        &mut self,
        market_index: u16,
        market_type: &MarketType,
        slot: u64,
        _oracle_price_data: &OraclePriceData,
        node_type: NodeType,
    ) -> SdkResult<Vec<Node>> {
        // let market = match market_type {
        //     MarketType::Perp => self
        //         .exchange
        //         .perp
        //         .get_mut(&market_index)
        //         .ok_or(SdkError::Generic("does not find perp market".to_string()))?,
        //     MarketType::Spot => self
        //         .exchange
        //         .spot
        //         .get_mut(&market_index)
        //         .ok_or(SdkError::Generic("does not find spot market".to_string()))?,
        // };

        self.update_resting_limit_orders(slot);

        let best_orders = self.get_best_orders(market_type, SubType::Bid, node_type, market_index);

        Ok(best_orders)
    }

    fn get_taking_asks(
        &mut self,
        market_index: u16,
        market_type: &MarketType,
        slot: u64,
        _oracle_price_data: &OraclePriceData,
        node_type: NodeType,
    ) -> SdkResult<Vec<Node>> {
        // let market = match market_type {
        //     MarketType::Perp => self
        //         .exchange
        //         .perp
        //         .get_mut(&market_index)
        //         .ok_or(SdkError::Generic("does not find perp market".to_string()))?,
        //     MarketType::Spot => self
        //         .exchange
        //         .spot
        //         .get_mut(&market_index)
        //         .ok_or(SdkError::Generic("does not find spot market".to_string()))?,
        // };

        self.update_resting_limit_orders(slot);

        let best_orders = self.get_best_orders(market_type, SubType::Ask, node_type, market_index);

        Ok(best_orders)
    }

    pub fn get_best_orders(
        &self,
        market_type: &MarketType,
        sub_type: SubType,
        node_type: NodeType,
        market_index: u16,
    ) -> Vec<Node> {
        let market = match market_type {
            MarketType::Perp => self.exchange.perp.get_mut(&market_index).expect("market"),
            MarketType::Spot => self.exchange.spot.get_mut(&market_index).expect("market"),
        };
        let mut order_list = market.get_order_list_for_node_type(node_type);

        let mut best_orders: Vec<Node> = vec![];

        match sub_type {
            SubType::Bid => {
                while !order_list.bids_empty() {
                    if let Some(node) = order_list.get_best_bid() {
                        best_orders.push(node);
                    }
                }
            }
            SubType::Ask => {
                while !order_list.asks_empty() {
                    if let Some(node) = order_list.get_best_ask() {
                        best_orders.push(node);
                    }
                }
            }
            _ => unimplemented!(),
        }

        best_orders
    }

    fn get_resting_limit_asks(
        &mut self,
        slot: u64,
        market_type: &MarketType,
        market_index: u16,
        oracle_price_data: &OraclePriceData,
    ) -> Vec<Node> {
        self.update_resting_limit_orders(slot);

        let mut resting_limit_orders = self.get_best_orders(
            market_type,
            SubType::Ask,
            NodeType::RestingLimit,
            market_index,
        );
        let mut floating_limit_orders = self.get_best_orders(
            market_type,
            SubType::Ask,
            NodeType::FloatingLimit,
            market_index,
        );

        let comparative = Box::new(
            |node_a: &Node, node_b: &Node, slot: u64, oracle_price_data: &OraclePriceData| {
                node_a.get_price(oracle_price_data, slot)
                    > node_b.get_price(oracle_price_data, slot)
            },
        );

        let mut all_orders = vec![];
        all_orders.append(&mut resting_limit_orders);
        all_orders.append(&mut floating_limit_orders);

        all_orders.sort_by(|a, b| {
            if comparative(a, b, slot, oracle_price_data) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });

        all_orders
    }

    fn get_resting_limit_bids(
        &mut self,
        slot: u64,
        market_type: &MarketType,
        market_index: u16,
        oracle_price_data: &OraclePriceData,
    ) -> Vec<Node> {
        self.update_resting_limit_orders(slot);

        let mut resting_limit_orders = self.get_best_orders(
            market_type,
            SubType::Bid,
            NodeType::RestingLimit,
            market_index,
        );
        let mut floating_limit_orders = self.get_best_orders(
            market_type,
            SubType::Bid,
            NodeType::FloatingLimit,
            market_index,
        );

        let comparative = Box::new(
            |node_a: &Node, node_b: &Node, slot: u64, oracle_price_data: &OraclePriceData| {
                node_a.get_price(oracle_price_data, slot)
                    < node_b.get_price(oracle_price_data, slot)
            },
        );

        let mut all_orders = vec![];
        all_orders.append(&mut resting_limit_orders);
        all_orders.append(&mut floating_limit_orders);

        all_orders.sort_by(|a, b| {
            if comparative(a, b, slot, oracle_price_data) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });

        all_orders
    }

    /// Return `node`, single `marker_nodes`
    fn find_crossing_resting_limit_orders(
        &mut self,
        market_index: u16,
        slot: u64,
        market_type: &MarketType,
        oracle_price_data: &OraclePriceData,
    ) -> Vec<NodeToFill> {
        let mut nodes_to_fill = Vec::new();

        for ask_node in
            self.get_resting_limit_asks(slot, market_type, market_index, oracle_price_data)
        {
            for bid_node in
                self.get_resting_limit_bids(slot, market_type, market_index, oracle_price_data)
            {
                let bid_price = bid_node.get_price(oracle_price_data, slot);
                let ask_price = ask_node.get_price(oracle_price_data, slot);

                // orders don't cross
                if bid_price < ask_price {
                    break;
                }

                let bid_order = bid_node.get_order();
                let ask_order = ask_node.get_order();

                let same_user = bid_node.get_user_account() == ask_node.get_user_account();
                if same_user {
                    continue;
                }

                let maker_and_taker = self.determine_maker_and_taker(ask_node, bid_node);

                // unable to match maker and taker due to post only or slot
                if let Some((taker_node, maker_node)) = maker_and_taker {
                    let bid_base_remaining =
                        bid_order.base_asset_amount - bid_order.base_asset_amount_filled;
                    let ask_base_remaining =
                        ask_order.base_asset_amount - ask_order.base_asset_amount_filled;

                    let base_filled = std::cmp::min(bid_base_remaining, ask_base_remaining);

                    let mut new_bid_order = bid_order.clone();
                    new_bid_order.base_asset_amount_filled =
                        bid_order.base_asset_amount_filled + base_filled;

                    if let Some(mut orders) = self.get_list_for_order(&new_bid_order, slot) {
                        let (_sub_type, node_type) =
                            get_node_subtype_and_type(&new_bid_order, slot);
                        let order_node =
                            create_node(node_type, new_bid_order, bid_node.get_user_account());
                        orders.update_bid(order_node);
                    }

                    // ask completely filled
                    let mut new_ask_order = ask_order.clone();
                    new_ask_order.base_asset_amount_filled =
                        ask_order.base_asset_amount_filled + base_filled;

                    if let Some(mut orders) = self.get_list_for_order(&new_ask_order, slot) {
                        let (_sub_type, node_type) =
                            get_node_subtype_and_type(&new_ask_order, slot);
                        let order_node =
                            create_node(node_type, new_ask_order, ask_node.get_user_account());
                        orders.update_bid(order_node);
                    }

                    nodes_to_fill.push(NodeToFill::new(taker_node, vec![maker_node]));

                    if new_ask_order.base_asset_amount == new_ask_order.base_asset_amount_filled {
                        break;
                    }
                }
            }
        }

        nodes_to_fill
    }

    fn determine_maker_and_taker(&self, ask_node: Node, bid_node: Node) -> Option<(Node, Node)> {
        let ask_order = ask_node.get_order();
        let ask_slot = ask_order.slot + ask_order.auction_duration as u64;

        let bid_order = bid_node.get_order();
        let bid_slot = bid_order.slot + bid_order.auction_duration as u64;

        if bid_order.post_only && ask_order.post_only {
            return None;
        } else if bid_order.post_only {
            return Some((ask_node, bid_node));
        } else if ask_order.post_only {
            return Some((bid_node, ask_node));
        } else if ask_slot < bid_slot {
            return Some((bid_node, ask_node));
        } else {
            return Some((ask_node, bid_node));
        }
    }

    pub fn find_nodes_to_trigger(
        &self,
        market_index: u16,
        oracle_price: u64,
        market_type: MarketType,
        state_account: Arc<std::sync::RwLock<State>>,
    ) -> Vec<Node> {
        let state_account = state_account.read().unwrap();
        if state_account.exchange_status != ExchangeStatus::active() {
            return vec![];
        }

        let mut nodes_to_trigger = Vec::new();
        let market_nodes_list = match market_type {
            MarketType::Perp => &self.exchange.perp,
            MarketType::Spot => &self.exchange.spot,
        };
        if let Some(market) = market_nodes_list.get(&market_index) {
            for node in &market.trigger_orders.bids {
                if oracle_price > node.node.get_order().trigger_price {
                    nodes_to_trigger.push(node.node);
                } else {
                    break;
                }
            }

            for node in &market.trigger_orders.asks {
                if oracle_price < node.node.get_order().trigger_price {
                    nodes_to_trigger.push(node.node);
                } else {
                    break;
                }
            }
        }

        nodes_to_trigger
    }

    pub fn get_l2<T>(
        &mut self,
        market_index: u16,
        market_type: &MarketType,
        slot: u64,
        oracle_price_data: &OraclePriceData,
        depth: usize,
        fallback_l2_generators: &mut [Box<dyn L2OrderBookGenerator>],
    ) -> L2OrderBook {
        let maker_ask_l2_level_generator = get_l2_generator_from_dlob_nodes(
            self.get_resting_limit_asks(slot, market_type, market_index, oracle_price_data)
                .into_iter(),
            *oracle_price_data,
            slot,
        );

        let fallback_ask_generators: Vec<_> = fallback_l2_generators
            .iter_mut()
            .map(|generator| generator.get_l2_asks())
            .collect();

        let mut l2_level_generators = vec![maker_ask_l2_level_generator];
        l2_level_generators.extend(fallback_ask_generators);
        let ask_l2_level_generator =
            merge_l2_level_generators(l2_level_generators, |a, b| a.price < b.price);

        let asks = create_l2_levels(ask_l2_level_generator, depth);

        let maker_bid_generator = get_l2_generator_from_dlob_nodes(
            self.get_resting_limit_bids(slot, market_type, market_index, oracle_price_data)
                .into_iter(),
            *oracle_price_data,
            slot,
        );

        let fallback_bid_generators: Vec<_> = fallback_l2_generators
            .iter_mut()
            .map(|generator| generator.get_l2_bids())
            .collect();

        let mut l2_level_generators = vec![maker_bid_generator];
        l2_level_generators.extend(fallback_bid_generators);
        let bid_l2_level_generator =
            merge_l2_level_generators(l2_level_generators, |a, b| a.price > b.price);

        let bids = create_l2_levels(bid_l2_level_generator, depth);

        L2OrderBook { asks, bids, slot }
    }

    pub fn get_l3(
        &mut self,
        market_index: u16,
        market_type: &MarketType,
        slot: u64,
        oracle_price_data: &OraclePriceData,
    ) -> L3OrderBook {
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        let resting_asks =
            self.get_resting_limit_asks(slot, market_type, market_index, oracle_price_data);

        for ask in resting_asks {
            asks.push(L3Level {
                price: ask.get_price(oracle_price_data, slot),
                size: ask
                    .get_order()
                    .base_asset_amount
                    .sub(ask.get_order().base_asset_amount_filled),
                maker: ask.get_user_account(),
                order_id: ask.get_order().order_id,
            });
        }

        let resting_bids =
            self.get_resting_limit_bids(slot, market_type, market_index, oracle_price_data);

        for bid in resting_bids {
            bids.push(L3Level {
                price: bid.get_price(oracle_price_data, slot),
                size: bid
                    .get_order()
                    .base_asset_amount
                    .sub(bid.get_order().base_asset_amount_filled),
                maker: bid.get_user_account(),
                order_id: bid.get_order().order_id,
            });
        }

        L3OrderBook { asks, bids, slot }
    }
}

impl Default for DLOB {
    fn default() -> Self {
        Self::new()
    }
}

impl Event for DLOB {
    fn box_clone(&self) -> Box<dyn Event> {
        Box::new((*self).clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drift::{
        math::constants::PRICE_PRECISION_U64,
        state::user::{Order, OrderType},
    };
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_dlob_insert() {
        let dlob = DLOB::new();
        let user_account = Pubkey::new_unique();
        let taking_limit_order = Order {
            order_id: 1,
            slot: 1,
            market_index: 0,
            market_type: MarketType::Perp,
            ..Order::default()
        };
        let floating_limit_order = Order {
            order_id: 2,
            oracle_price_offset: 1,
            market_index: 0,
            market_type: MarketType::Perp,
            ..Order::default()
        };
        let resting_limit_order = Order {
            order_id: 3,
            slot: 3,
            market_index: 0,
            market_type: MarketType::Perp,
            ..Order::default()
        };
        let market_order = Order {
            order_id: 4,
            slot: 4,
            market_index: 0,
            market_type: MarketType::Perp,
            ..Order::default()
        };
        let trigger_order = Order {
            order_id: 5,
            slot: 5,
            market_index: 0,
            market_type: MarketType::Perp,
            ..Order::default()
        };

        dlob.insert_order(&taking_limit_order, user_account, 1);
        dlob.insert_order(&floating_limit_order, user_account, 0);
        dlob.insert_order(&resting_limit_order, user_account, 3);
        dlob.insert_order(&market_order, user_account, 4);
        dlob.insert_order(&trigger_order, user_account, 5);

        assert!(dlob.get_order(1, user_account).is_some());
        assert!(dlob.get_order(2, user_account).is_some());
        assert!(dlob.get_order(3, user_account).is_some());
        assert!(dlob.get_order(4, user_account).is_some());
        assert!(dlob.get_order(5, user_account).is_some());
    }

    #[test]
    fn test_dlob_ordering() {
        let dlob = DLOB::new();

        let user_account = Pubkey::new_unique();
        let order_1 = Order {
            order_id: 1,
            slot: 1,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };
        let order_2 = Order {
            order_id: 2,
            slot: 2,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };
        let order_3 = Order {
            order_id: 3,
            slot: 3,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };
        let order_4 = Order {
            order_id: 4,
            slot: 4,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };
        let order_5 = Order {
            order_id: 5,
            slot: 5,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };

        dlob.insert_order(&order_1, user_account, 1);
        dlob.insert_order(&order_2, user_account, 2);
        dlob.insert_order(&order_3, user_account, 3);
        dlob.insert_order(&order_4, user_account, 4);
        dlob.insert_order(&order_5, user_account, 5);

        assert!(dlob.get_order(1, user_account).is_some());
        assert!(dlob.get_order(2, user_account).is_some());
        assert!(dlob.get_order(3, user_account).is_some());
        assert!(dlob.get_order(4, user_account).is_some());
        assert!(dlob.get_order(5, user_account).is_some());

        let best_orders =
            dlob.get_best_orders(&MarketType::Perp, SubType::Bid, NodeType::TakingLimit, 0);

        assert_eq!(best_orders[0].get_order().slot, 1);
        assert_eq!(best_orders[1].get_order().slot, 2);
        assert_eq!(best_orders[2].get_order().slot, 3);
        assert_eq!(best_orders[3].get_order().slot, 4);
        assert_eq!(best_orders[4].get_order().slot, 5);
    }

    #[test]
    fn test_update_resting_limit_orders() {
        let mut dlob = DLOB::new();

        let user_account = Pubkey::new_unique();
        let order_1 = Order {
            order_id: 1,
            slot: 1,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            auction_duration: 1,
            ..Order::default()
        };

        dlob.insert_order(&order_1, user_account, 1);

        let markets_for_market_type = dlob.exchange.perp.clone();
        let market = markets_for_market_type.get(&0).unwrap();

        assert_eq!(market.taking_limit_orders.bids.len(), 1);

        let slot = 5;

        drop(market);
        drop(markets_for_market_type);

        dlob.update_resting_limit_orders(slot);

        let markets_for_market_type = dlob.exchange.perp.clone();
        let market = markets_for_market_type.get(&0).unwrap();

        assert_eq!(market.taking_limit_orders.bids.len(), 0);
        assert_eq!(market.resting_limit_orders.bids.len(), 1);
    }

    #[test]
    fn test_get_resting_limit_asks() {
        let mut dlob = DLOB::new();

        let v_ask = 15;
        let v_bid = 10;

        let oracle_price_data = OraclePriceData {
            price: (v_bid + v_ask) / 2,
            confidence: 1,
            delay: 0,
            has_sufficient_number_of_data_points: true,
        };

        let user_account = Pubkey::new_unique();
        let order_1 = Order {
            order_id: 1,
            slot: 1,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Short,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 11 * PRICE_PRECISION_U64,
            ..Order::default()
        };

        let order_2 = Order {
            order_id: 2,
            slot: 11,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Short,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 12 * PRICE_PRECISION_U64,
            ..Order::default()
        };

        let order_3 = Order {
            order_id: 3,
            slot: 21,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Short,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 13 * PRICE_PRECISION_U64,
            ..Order::default()
        };

        dlob.insert_order(&order_1, user_account, 1);
        dlob.insert_order(&order_2, user_account, 11);
        dlob.insert_order(&order_3, user_account, 21);

        let mut slot = 1;

        dbg!("expecting 0");
        let resting_limit_asks =
            dlob.get_resting_limit_asks(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_asks.len(), 0);

        slot += 11;

        dbg!("expecting 1");
        let resting_limit_asks =
            dlob.get_resting_limit_asks(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_asks.len(), 1);
        assert_eq!(resting_limit_asks[0].get_order().order_id, 1);

        slot += 11;

        dbg!("expecting 2");
        let resting_limit_asks =
            dlob.get_resting_limit_asks(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_asks.len(), 2);
        assert_eq!(resting_limit_asks[0].get_order().order_id, 1);
        assert_eq!(resting_limit_asks[1].get_order().order_id, 2);

        slot += 11;

        dbg!("expecting 3");
        let resting_limit_asks =
            dlob.get_resting_limit_asks(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_asks.len(), 3);
        assert_eq!(resting_limit_asks[0].get_order().order_id, 1);
        assert_eq!(resting_limit_asks[1].get_order().order_id, 2);
        assert_eq!(resting_limit_asks[2].get_order().order_id, 3);
    }

    #[test]
    fn test_get_resting_limit_bids() {
        let mut dlob = DLOB::new();

        let v_ask = 15;
        let v_bid = 10;

        let oracle_price_data = OraclePriceData {
            price: (v_bid + v_ask) / 2,
            confidence: 1,
            delay: 0,
            has_sufficient_number_of_data_points: true,
        };

        let user_account = Pubkey::new_unique();
        let order_1 = Order {
            order_id: 1,
            slot: 1,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 11,
            ..Order::default()
        };

        let order_2 = Order {
            order_id: 2,
            slot: 11,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 12,
            ..Order::default()
        };

        let order_3 = Order {
            order_id: 3,
            slot: 21,
            market_index: 0,
            direction: drift::controller::position::PositionDirection::Long,
            market_type: MarketType::Perp,
            order_type: OrderType::Limit,
            auction_duration: 10,
            price: 13,
            ..Order::default()
        };

        dlob.insert_order(&order_1, user_account, 1);
        dlob.insert_order(&order_2, user_account, 11);
        dlob.insert_order(&order_3, user_account, 21);

        let mut slot = 1;

        dbg!("expecting 0");
        let resting_limit_bids =
            dlob.get_resting_limit_bids(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_bids.len(), 0);

        slot += 11;

        dbg!("expecting 1");
        let resting_limit_bids =
            dlob.get_resting_limit_bids(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_bids.len(), 1);
        assert_eq!(resting_limit_bids[0].get_order().order_id, 1);

        slot += 11;

        dbg!("expecting 2");
        let resting_limit_bids =
            dlob.get_resting_limit_bids(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_bids.len(), 2);
        assert_eq!(resting_limit_bids[0].get_order().order_id, 2);
        assert_eq!(resting_limit_bids[1].get_order().order_id, 1);

        slot += 11;

        dbg!("expecting 3");
        let resting_limit_bids =
            dlob.get_resting_limit_bids(slot, &MarketType::Perp, 0, &oracle_price_data);

        assert_eq!(resting_limit_bids.len(), 3);
        assert_eq!(resting_limit_bids[0].get_order().order_id, 3);
        assert_eq!(resting_limit_bids[1].get_order().order_id, 2);
        assert_eq!(resting_limit_bids[2].get_order().order_id, 1);
    }
}
