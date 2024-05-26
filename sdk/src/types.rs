use std::{
    cell::{BorrowError, BorrowMutError},
    cmp::Ordering,
    sync::{Arc, Mutex},
};

use anchor_lang::AccountDeserialize;
use borsh::BorshDeserialize;
use drift::{
    error::ErrorCode,
    state::user::{MarketType, UserFees, UserStats},
};
use futures_util::Sink;
use solana_sdk::{
    instruction::{AccountMeta, InstructionError},
    pubkey::Pubkey,
    transaction::TransactionError,
};
use thiserror::Error;
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

use crate::event_emitter::Event;

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
pub struct SinkError(
    pub <WebSocketStream<MaybeTlsStream<TcpStream>> as Sink<tungstenite::Message>>::Error,
);

impl std::fmt::Display for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocket Sink Error: {}", self.0)
    }
}

impl std::error::Error for SinkError {}

impl From<SinkError> for SdkError {
    fn from(err: SinkError) -> Self {
        SdkError::SubscriptionFailure(err)
    }
}

impl From<drift::error::ErrorCode> for SdkError {
    fn from(value: drift::error::ErrorCode) -> Self {
        Self::DriftProgramError(value)
    }
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("{0}")]
    Rpc(#[from] solana_client::client_error::ClientError),
    #[error("{0}")]
    Ws(#[from] solana_client::nonblocking::pubsub_client::PubsubClientError),
    #[error("{0}")]
    Anchor(#[from] Box<anchor_lang::error::Error>),
    #[error("error while deserializing")]
    Deserializing,
    #[error("invalid drift account")]
    InvalidAccount,
    #[error("invalid oracle account")]
    InvalidOracle,
    #[error("invalid keypair seed")]
    InvalidSeed,
    #[error("invalid base58 value")]
    InvalidBase58,
    #[error("user does not have position: {0}")]
    NoPosiiton(u16),
    #[error("insufficient SOL balance for fees")]
    OutOfSOL,
    #[error("{0}")]
    Signing(#[from] solana_sdk::signer::SignerError),
    #[error("WebSocket connection failed {0}")]
    ConnectionError(#[from] tungstenite::Error),
    #[error("Subscription failure: {0}")]
    SubscriptionFailure(SinkError),
    #[error("Received Error from websocket")]
    WebsocketError,
    #[error("Missed DLOB heartbeat")]
    MissedHeartbeat,
    #[error("Unsupported account data format")]
    UnsupportedAccountData,
    #[error("Could not decode data: {0}")]
    CouldntDecode(#[from] base64::DecodeError),
    #[error("Couldn't join task: {0}")]
    CouldntJoin(#[from] tokio::task::JoinError),
    #[error("Couldn't send unsubscribe message: {0}")]
    CouldntUnsubscribe(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error("MathError")]
    MathError(String),
    #[error("{0}")]
    BorrowMutError(#[from] BorrowMutError),
    #[error("{0}")]
    BorrowError(#[from] BorrowError),
    #[error("{0}")]
    Generic(String),
    #[error("max connection attempts reached")]
    MaxReconnectionAttemptsReached,
    #[error("jit taker order not found")]
    JitOrderNotFound,
    #[error("Drift Program occured. Error Code: {0}")]
    DriftProgramError(drift::error::ErrorCode),
}

impl SdkError {
    /// extract anchor error code from the SdkError if it exists
    pub fn to_anchor_error_code(&self) -> Option<ErrorCode> {
        if let SdkError::Rpc(inner) = self {
            if let Some(TransactionError::InstructionError(_, InstructionError::Custom(code))) =
                inner.get_transaction_error()
            {
                // inverse of anchor's 'From<ErrorCode> for u32'
                return Some(unsafe {
                    std::mem::transmute(code - anchor_lang::error::ERROR_CODE_OFFSET)
                });
            }
        }
        None
    }
    /// convert to 'out of sol' error is possible
    pub fn to_out_of_sol_error(&self) -> Option<SdkError> {
        if let SdkError::Rpc(inner) = self {
            if let Some(
                TransactionError::InsufficientFundsForFee
                | TransactionError::InsufficientFundsForRent { account_index: _ },
            ) = inner.get_transaction_error()
            {
                return Some(Self::OutOfSOL);
            }
        }
        None
    }
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
        Box::new(*self).clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// #[derive(Eq, PartialEq, Debug, BorshDeserialize)]
// #[repr(C)]
// pub struct UserStatsAccount {
//     /// The authority for all of a users sub accounts
//     pub authority: Pubkey,
//     /// The address that referred this user
//     pub referrer: Pubkey,
//     /// Stats on the fees paid by the user
//     pub fees: UserFees,
//
//     /// The timestamp of the next epoch
//     /// Epoch is used to limit referrer rewards earned in single epoch
//     pub next_epoch_ts: i64,
//
//     /// Rolling 30day maker volume for user
//     /// precision: QUOTE_PRECISION
//     pub maker_volume_30d: u64,
//     /// Rolling 30day taker volume for user
//     /// precision: QUOTE_PRECISION
//     pub taker_volume_30d: u64,
//     /// Rolling 30day filler volume for user
//     /// precision: QUOTE_PRECISION
//     pub filler_volume_30d: u64,
//     /// last time the maker volume was updated
//     pub last_maker_volume_30d_ts: i64,
//     /// last time the taker volume was updated
//     pub last_taker_volume_30d_ts: i64,
//     /// last time the filler volume was updated
//     pub last_filler_volume_30d_ts: i64,
//
//     /// The amount of tokens staked in the quote spot markets if
//     pub if_staked_quote_asset_amount: u64,
//     /// The current number of sub accounts
//     pub number_of_sub_accounts: u16,
//     /// The number of sub accounts created. Can be greater than the number of sub accounts if user
//     /// has deleted sub accounts
//     pub number_of_sub_accounts_created: u16,
//     /// Whether the user is a referrer. Sub account 0 can not be deleted if user is a referrer
//     pub is_referrer: bool,
//     pub disable_update_perp_bid_ask_twap: bool,
//     pub padding: [u8; 50],
// }
