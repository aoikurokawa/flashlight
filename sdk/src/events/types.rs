use drift::state::events::{
    CurveRecord, DepositRecord, FundingPaymentRecord, FundingRateRecord, InsuranceFundRecord,
    InsuranceFundStakeRecord, LPRecord, LiquidationRecord, NewUserRecord, OrderActionRecord,
    OrderRecord, SettlePnlRecord, SpotInterestRecord, SwapRecord,
};
use solana_sdk::signature::Signature;

pub struct Event<E> {
    pub tx_sig: Signature,

    pub slot: u64,

    /// Unique index for each event inside a tx
    pub tx_sig_index: u64,

    pub data: E,
}

pub struct WrappedEvent<E> {
    pub event: Event<E>,
    pub event_type: EventMap,
}
pub enum EventMap {
    DepositRecord(Event<DepositRecord>),
    FundingPaymentRecord(Event<FundingPaymentRecord>),
    LiquidationRecord(Event<LiquidationRecord>),
    FundingRateRecord(Event<FundingRateRecord>),
    OrderRecord(Event<OrderRecord>),
    OrderActionRecord(Event<OrderActionRecord>),
    SettlePnlRecord(Event<SettlePnlRecord>),
    NewUserRecord(Event<NewUserRecord>),
    LPRecord(Event<LPRecord>),
    InsuranceFundRecord(Event<InsuranceFundRecord>),
    SpotInterestRecord(Event<SpotInterestRecord>),
    InsuranceFundStakeRecord(Event<InsuranceFundStakeRecord>),
    CurveRecord(Event<CurveRecord>),
    SwapRecord(Event<SwapRecord>),
}

// pub enum EventType {
//     DepositRecord,
//     FundingPaymentRecord,
//     LiquidationRecord,
//     FundingRateRecord,
//     OrderRecord,
//     OrderActionRecord,
//     SettlePnlRecord,
//     NewUserRecord,
//     LPRecord,
//     InsuranceFundRecord,
//     SpotInterestRecord,
//     InsuranceFundStakeRecord,
//     CurveRecord,
//     SwapRecord,
// }
