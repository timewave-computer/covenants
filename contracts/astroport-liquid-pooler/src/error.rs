use cosmwasm_std::{DecimalRangeExceeded, OverflowError, StdError};
use covenant_utils::op_mode::ContractOperationError;
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

    #[error(transparent)]
    ContractOperationError(#[from] ContractOperationError),

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

    #[error("Pair type mismatch")]
    PairTypeMismatch {},

    #[error("Only holder can withdraw the position")]
    NotHolder {},

    #[error("no covenant denom or lp tokens available")]
    NothingToWithdraw {},

    #[error("Withdraw percentage range must belong to range (0.0, 1.0]")]
    WithdrawPercentageRangeError {},
}
