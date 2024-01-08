use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("A withdraw process already started")]
    WithdrawAlreadyStarted {},

    #[error("No withdrawer address configured")]
    NoWithdrawer {},

    #[error("No withdraw_to address configured")]
    NoWithdrawTo {},

    #[error("The position is still locked, unlock at: {0}")]
    LockupPeriodNotOver(String),

    #[error("The lockup period is already expired")]
    LockupPeriodIsExpired,

    #[error("The lockup period must be in the future")]
    MustBeFutureLockupPeriod,

    #[error("We exepct 2 denoms to be recieved by the pooler")]
    InvalidFunds,
}
