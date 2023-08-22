
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("expiry block is already past")]
    InvalidExpiryBlockHeight {},

    #[error("lockup validation failed")]
    LockupValidationError {},

    #[error("shares of covenant parties must add up to 1.0")]
    InvolvedPartiesConfigError {},

    #[error("ragequit is disabled")]
    RagequitDisabled {},

    #[error("only covenant parties can initiate ragequit")]
    RagequitUnauthorized {},

    #[error("ragequit attempt with lockup period passed")]
    RagequitWithLockupPassed {},

    #[error("ragequit already active")]
    RagequitAlreadyActive {},

    #[error("no lp tokens available")]
    NoLpTokensAvailable {},
}