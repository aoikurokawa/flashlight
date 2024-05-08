
use thiserror::Error;


pub mod jit_maker;
pub mod utils;

pub type JitResult<T> = Result<T, JitError>;

#[derive(Debug, Error)]
pub enum JitError {
    #[error("{0}")]
    SdkError(#[from] drift_sdk::types::SdkError),
    #[error("{0}")]
    JitterError(#[from] rust::types::JitError),
    #[error("{0}")]
    Generic(String),
}
