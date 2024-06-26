use std::collections::HashMap;

use sdk::dlob::dlob_node::{DLOBNode, Node};
use solana_sdk::pubkey::Pubkey;

const PROBABILITY_PRECISION: u64 = 1000;

pub fn select_makers(maker_node_map: &HashMap<Pubkey, Vec<Node>>) -> HashMap<Pubkey, Vec<Node>> {
    let selected_makers = HashMap::new();

    selected_makers
}

fn get_probability(dlob_nodes: &[Node], total_liquidity: u64) -> u64 {
    let maker_liquidity = get_maker_liquidity(dlob_nodes);
    maker_liquidity * PROBABILITY_PRECISION
}

fn get_maker_liquidity(dlob_nodes: &[Node]) -> u64 {
    dlob_nodes.iter().fold(0, |acc, dlob_node| {
        let order = dlob_node.get_order();
        acc + order.base_asset_amount - order.base_asset_amount_filled
    })
}
