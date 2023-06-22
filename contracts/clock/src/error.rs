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

    #[error("the contract is paused")]
    Paused {},

    #[error("the contract is not paused")]
    NotPaused {},

    #[error("tick max gas must be non-zero")]
    ZeroTickMaxGas {},

    #[error("only contracts may be enqueued. error reading contract info: ({0})")]
    NotContract(String),
}
