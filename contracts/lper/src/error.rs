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

    #[error("zero expected native token amount can result in division by 0")]
    ZeroExpectedNativeTokenAmountError {},
}
