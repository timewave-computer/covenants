use cosmwasm_std::{CheckedMultiplyRatioError, OverflowError, StdError};
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    CheckedMultiplyError(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    NeutronError(#[from] NeutronError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Not clock")]
    ClockVerificationError {},

    #[error("Unknown holder address. Migrate update to set it.")]
    MissingHolderError {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Osmosis pool error: {0}")]
    OsmosisPoolError(String),

    #[error("Fund deposit error: expected {0} bal {1}, got {2}")]
    FundsDepositError(String, String, String),

    #[error("state machine: {0}")]
    StateMachineError(String),

    #[error("polytone error: {0}")]
    PolytoneError(String),

    #[error("Only holder can withdraw the position")]
    NotHolder {},
}

impl From<ContractError> for NeutronError {
    fn from(value: ContractError) -> Self {
        NeutronError::Std(StdError::generic_err(value.to_string()))
    }
}
