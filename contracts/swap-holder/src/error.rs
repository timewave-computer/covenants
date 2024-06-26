use cosmwasm_std::StdError;
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
    ContractOperationError(#[from] ContractOperationError),

    #[error("No withdrawer address configured")]
    NoWithdrawerError {},

    #[error("Insufficient funds to forward")]
    InsufficientFunds {},

    #[error("unexpected reply id")]
    UnexpectedReplyId {},

    #[error("Lockup config must be in the future")]
    LockupConfigValidationError {},
}
