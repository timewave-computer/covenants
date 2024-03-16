use cosmwasm_std::{Instantiate2AddressError, StdError};
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Attempt to deposit zero")]
    ZeroDeposit {},

    #[error("Unknown reply id")]
    UnknownReplyId {},

    #[error("SubMsg reply error")]
    ReplyError { err: String },

    #[error("Failed to instantiate {contract:?} contract")]
    ContractInstantiationError {
        contract: String,
        err: ParseReplyError,
    },

    #[error("{0}")]
    InstantiationError(#[from] Instantiate2AddressError),

    #[error("{0} contribution missing an explicit split configuration (got {1})")]
    DenomMisconfigurationError(String,String),
}
