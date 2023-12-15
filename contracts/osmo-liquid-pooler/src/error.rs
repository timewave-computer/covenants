use cosmwasm_std::{OverflowError, StdError};
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    NeutronError(#[from] NeutronError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Not clock")]
    ClockVerificationError {},

    #[error("Unknown holder address. Migrate update to set it.")]
    MissingHolderError {},
}
