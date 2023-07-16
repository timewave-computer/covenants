use cosmwasm_std::{StdError, Instantiate2AddressError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Attempt to deposit zero")]
    ZeroDeposit {},

    #[error("Unknown reply id")]
    UnknownReplyId {},

    #[error("SubMsg reply error")]
    ReplyError { err: String },

    #[error("Failed to instantiate {contract:?} contract")]
    ContractInstantiationError { contract: String },
}
