use cosmwasm_std::{DecimalRangeExceeded, OverflowError, StdError};
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    NeutronError(#[from] NeutronError),

    #[error(transparent)]
    OverflowError(#[from] OverflowError),

    #[error(transparent)]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Not clock")]
    ClockVerificationError {},

    #[error("Single side LP limit exceeded")]
    SingleSideLpLimitError {},

    #[error("Non zero balances for single side liquidity")]
    SingleSideLpNonZeroBalanceError {},

    #[error("Zero balance for double side liquidity")]
    DoubleSideLpZeroBalanceError {},

    #[error("Insufficient funds for double sided LP")]
    DoubleSideLpLimitError {},

    #[error("Incomplete pool assets")]
    IncompletePoolAssets {},

    #[error("Pool validation error")]
    PoolValidationError {},

    #[error("Price range error")]
    PriceRangeError {},

    #[error("Unknown holder address. Migrate update to set it.")]
    MissingHolderError {},

    #[error("Pair type mismatch")]
    PairTypeMismatch {},

    #[error("Only holder can withdraw the position")]
    NotHolder {},

    #[error("no lp tokens available")]
    NoLpTokensAvailable {},
}
