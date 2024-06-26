use std::collections::HashMap;

use sdk::{
    dlob::dlob_node::{DLOBNode, Node},
    math::util::div_ceil,
};
use solana_sdk::pubkey::Pubkey;

use crate::filler::MAX_MAKERS_PER_FILL;

const PROBABILITY_PRECISION: u64 = 1000;

pub fn select_makers(maker_node_map: &HashMap<Pubkey, Vec<Node>>) -> HashMap<Pubkey, Vec<Node>> {
    let mut selected_makers = HashMap::new();

    while selected_makers.len() < MAX_MAKERS_PER_FILL && !maker_node_map.is_empty() {
        let maker = select_maker(maker_node_map);
        match maker {
            Some(maker) => {
                if let Some(maker_nodes) = maker_node_map.get(&maker) {
                    selected_makers.insert(maker, maker_nodes.to_vec());
                    maker_node_map.remove(&maker);
                }
            }
            None => break,
        }
    }

    selected_makers
}

fn select_maker(maker_node_map: &HashMap<Pubkey, Vec<Node>>) -> Option<Pubkey> {
    if maker_node_map.is_empty() {
        return None;
    }

    let mut total_liquidity = 0;
    for dlob_nodes in maker_node_map.values() {
        total_liquidity += get_maker_liquidity(dlob_nodes);
    }

    let mut probabilities = Vec::new();
    for dlob_nodes in maker_node_map.values() {
        probabilities.push(get_probability(dlob_nodes, total_liquidity));
    }

    let mut maker_index = 0;
    let random: u64 = rand::random();
    let mut sum = 0;
    for i in 0..probabilities.len() {
        sum += probabilities[i];
        if random < sum {
            maker_index = i;
            break;
        }
    }

    let keys: Vec<&Pubkey> = maker_node_map.keys().collect();
    keys.get(maker_index).copied().copied()
}

fn get_probability(dlob_nodes: &[Node], total_liquidity: u64) -> u64 {
    let maker_liquidity = get_maker_liquidity(dlob_nodes);
    div_ceil(maker_liquidity * PROBABILITY_PRECISION, total_liquidity)
}

fn get_maker_liquidity(dlob_nodes: &[Node]) -> u64 {
    dlob_nodes.iter().fold(0, |acc, dlob_node| {
        let order = dlob_node.get_order();
        acc + order.base_asset_amount - order.base_asset_amount_filled
    })
}
