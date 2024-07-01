use std::time::Instant;

use sdk::dlob::dlob::NodeToFill;

#[derive(Debug, Clone)]
pub(crate) enum TxType {
    Fill,
    Trigger,
    SettlePnl,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingTxSigsToconfirm {
    pub(crate) ts: Instant,
    pub(crate) node_filled: Vec<NodeToFill>,
    pub(crate) fill_tx_id: u16,
    pub(crate) tx_type: TxType,
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
