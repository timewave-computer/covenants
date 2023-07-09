use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Not clock")]
    ClockVerificationError {},

    #[error("Single side LP limit exceeded")]
    SingleSideLpLimitError {},
}
