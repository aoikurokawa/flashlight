use drift::state::{
    paused_operations::{PerpOperation, SpotOperation},
    state::{ExchangeStatus, State as StateAccount},
};

use crate::{dlob::dlob::MarketAccount, types::InsuranceFundOperation};

#[derive(Debug)]
pub enum Operation {
    PerpOperation(PerpOperation),
    SpotOperation(SpotOperation),
    InsuranceFundOperation(InsuranceFundOperation),
}

pub fn fill_paused(state_account: &StateAccount, market: &MarketAccount) -> bool {
    if (state_account.exchange_status & ExchangeStatus::FillPaused as u8)
        == ExchangeStatus::FillPaused as u8
    {
        return true;
    }

    match market {
        MarketAccount::PerpMarket(perp) => is_operation_paused(
            perp.paused_operations,
            Operation::PerpOperation(PerpOperation::Fill),
        ),
        MarketAccount::SpotMarket(spot) => is_operation_paused(
            spot.paused_operations,
            Operation::SpotOperation(SpotOperation::Fill),
        ),
    }
}

pub fn is_operation_paused(paused_operations: u8, operation: Operation) -> bool {
    match operation {
        Operation::PerpOperation(perp) => (paused_operations & perp as u8) > 0,
        Operation::SpotOperation(spot) => (paused_operations & spot as u8) > 0,
        Operation::InsuranceFundOperation(insurance) => (paused_operations & insurance as u8) > 0,
    }
}
