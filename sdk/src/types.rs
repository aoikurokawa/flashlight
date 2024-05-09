use std::cell::{BorrowError, BorrowMutError};

use drift::ErrorCode;
use futures_util::Sink;
use solana_sdk::{instruction::InstructionError, transaction::TransactionError};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

pub type SdkResult<T> = Result<T, SdkError>;

/// Drift program context
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Context {
    /// Target DevNet
    DevNet,
    /// Target MaiNnet
    MainNet,
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
