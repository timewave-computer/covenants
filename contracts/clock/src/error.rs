use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("sender is already in the queue")]
    AlreadyEnqueued,

    #[error("received an unexpected reply ID ({0})")]
    UnexpectedReplyId(u64),
}
