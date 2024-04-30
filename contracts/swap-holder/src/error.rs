use cosmwasm_std::StdError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    NeutronError(#[from] NeutronError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No withdrawer address configured")]
    NoWithdrawerError {},

    #[error("Insufficient funds to forward")]
    InsufficientFunds {},

    #[error("unexpected reply id")]
    UnexpectedReplyId {},
}
