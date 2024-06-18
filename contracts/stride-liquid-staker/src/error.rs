use cosmwasm_std::StdError;
use neutron_sdk::NeutronError;
use thiserror::Error;

use crate::msg::{ContractState, ExecuteMsg};

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("State machine error: cannot perform {0:?} from {1:?} state")]
    StateMachineError(ExecuteMsg, ContractState),

    #[error("Next contract is not ready for receiving the funds yet")]
    NextContractError {},

    #[error("Unsupported reply id: {0}")]
    UnsupportedReplyIdError(u64),
}

impl From<ContractError> for NeutronError {
    fn from(value: ContractError) -> Self {
        NeutronError::Std(StdError::generic_err(value.to_string()))
    }
}

impl From<ContractError> for StdError {
    fn from(value: ContractError) -> Self {
        StdError::generic_err(value.to_string())
    }
}
