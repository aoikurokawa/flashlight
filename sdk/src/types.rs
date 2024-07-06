use std::cmp::Ordering;

use anchor_lang::AccountDeserialize;
use drift::state::user::{MarketType, Order, User, UserStats};
use serde::Deserialize;
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

use crate::{error::SdkError, event_emitter::Event};

pub type SdkResult<T> = Result<T, SdkError>;

pub fn is_one_of_variant<T: PartialEq>(value: &T, variants: &[T]) -> bool {
    variants.iter().any(|variant| value == variant)
}

/// Drift program context
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Context {
    /// Target DevNet
    DevNet,
    /// Target MaiNnet
    MainNet,
}

#[derive(Debug, Clone)]
pub struct DataAndSlot<T>
where
    T: AccountDeserialize,
{
    pub slot: u64,
    pub data: T,
}

/// Id of a Drift market
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MarketId {
    pub(crate) index: u16,
    pub(crate) kind: MarketType,
}

impl MarketId {
    /// Id of a perp market
    pub const fn perp(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Perp,
        }
    }
    /// Id of a spot market
    pub const fn spot(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Spot,
        }
    }

    /// `MarketId` for the USDC Spot Market
    pub const QUOTE_SPOT: Self = Self {
        index: 0,
        kind: MarketType::Spot,
    };
}

impl From<(u16, MarketType)> for MarketId {
    fn from(value: (u16, MarketType)) -> Self {
        Self {
            index: value.0,
            kind: value.1,
        }
    }
}

#[derive(Debug)]
pub enum InsuranceFundOperation {
    Init = 1,
    Add = 2,
    RequestRemove = 4,
    Remove = 8,
}

/// Helper type for Accounts included in drift instructions
///
/// Provides sorting implementation matching drift program
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum RemainingAccount {
    Oracle { pubkey: Pubkey },
    Spot { pubkey: Pubkey, writable: bool },
    Perp { pubkey: Pubkey, writable: bool },
}

impl RemainingAccount {
    fn pubkey(&self) -> &Pubkey {
        match self {
            Self::Oracle { pubkey } => pubkey,
            Self::Spot { pubkey, .. } => pubkey,
            Self::Perp { pubkey, .. } => pubkey,
        }
    }
    fn parts(self) -> (Pubkey, bool) {
        match self {
            Self::Oracle { pubkey } => (pubkey, false),
            Self::Spot {
                pubkey, writable, ..
            } => (pubkey, writable),
            Self::Perp {
                pubkey, writable, ..
            } => (pubkey, writable),
        }
    }
    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

impl Ord for RemainingAccount {
    fn cmp(&self, other: &Self) -> Ordering {
        let type_order = self.discriminant().cmp(&other.discriminant());
        if let Ordering::Equal = type_order {
            self.pubkey().cmp(other.pubkey())
        } else {
            type_order
        }
    }
}

impl PartialOrd for RemainingAccount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<RemainingAccount> for AccountMeta {
    fn from(value: RemainingAccount) -> Self {
        let (pubkey, is_writable) = value.parts();
        AccountMeta {
            pubkey,
            is_writable,
            is_signer: false,
        }
    }
}

pub type UserStatsAccount = UserStats;

impl Event for UserStatsAccount {
    fn box_clone(&self) -> Box<dyn Event> {
        let event = Box::new(*self);
        event.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ReferrerInfo {
    pub referrer: Pubkey,
    pub referrer_stats: Pubkey,
}

#[derive(Debug, Clone, Deserialize)]
pub enum OracleSource {
    Pyth,
    Switchboard,
    QuoteAsset,
    Pyth1K,
    Pyth1M,
    PythStableCoin,
    Prelaunch,
}

#[derive(Default)]
pub struct BaseTxParams {
    pub compute_units: Option<u32>,
    pub compute_units_price: Option<u32>,
}

#[derive(Default)]
pub struct ProcessingTxParams {
    pub use_simulated_compute_units: Option<bool>,
    pub compute_units_buffer_multipler: Option<u64>,
    pub use_simulated_compute_units_for_cu_price_calculation: Option<bool>,
    pub get_cu_price_from_compute_units: Option<fn(u64) -> u64>,
}

#[derive(Default)]
pub struct TxParams {
    pub base: BaseTxParams,
    pub processing: ProcessingTxParams,
}

#[derive(Debug, Clone)]
pub struct MakerInfo {
    pub maker: Pubkey,
    pub maker_stats: Pubkey,
    pub maker_user_account: User,
    pub order: Option<Order>,
}

impl MakerInfo {
    pub fn new(
        maker: Pubkey,
        maker_stats: Pubkey,
        maker_user_account: User,
        order: Option<Order>,
    ) -> Self {
        Self {
            maker,
            maker_stats,
            maker_user_account,
            order,
        }
    }
}
