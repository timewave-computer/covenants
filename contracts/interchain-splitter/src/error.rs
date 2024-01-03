use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Caller is not the clock, only clock can tick contracts")]
    NotClock,

    #[error("misconfigured split")]
    SplitMisconfig {},

    #[error("unauthorized caller")]
    Unauthorized {},
}
