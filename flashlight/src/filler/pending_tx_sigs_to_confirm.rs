use std::time::Instant;

use sdk::dlob::dlob::NodeToFill;

pub(crate) enum TxType {
    Fill,
    Trigger,
    SettlePnl,
}

pub(crate) struct PendingTxSigsToconfirm {
    ts: Instant,
    node_filled: Vec<NodeToFill>,
    fill_tx_id: u16,
    tx_type: TxType,
}

impl PendingTxSigsToconfirm {
    pub fn new(ts: Instant, node_filled: &[NodeToFill], fill_tx_id: u16, tx_type: TxType) -> Self {
        Self {
            ts,
            node_filled: node_filled.to_vec(),
            fill_tx_id,
            tx_type,
        }
    }
}
