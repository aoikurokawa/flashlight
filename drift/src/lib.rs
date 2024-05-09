use anchor_lang::declare_id;
pub use controller::*;
pub use error::*;
pub use state::*;
pub use user::*;

pub mod controller;
pub mod error;
pub mod state;
pub mod user;

declare_id!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");
