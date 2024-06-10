use cosmwasm_std::StdError;
use covenant_utils::op_mode::ContractOperationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ContractOperationError(#[from] ContractOperationError),

    #[error("misconfigured split")]
    SplitMisconfig {},
}
