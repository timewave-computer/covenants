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
}

impl ContractError {
    pub fn to_std(&self) -> StdError {
        StdError::GenericErr {
            msg: self.to_string(),
        }
    }

    pub fn to_neutron_std(&self) -> NeutronError {
        NeutronError::Std(self.to_std())
    }
}
