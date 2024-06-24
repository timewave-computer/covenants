use cosmwasm_std::StdError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
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

    #[error("Caller is not whitelisted, can't enqueue")]
    NotWhitelisted,

    #[error("Must provide add or remove list")]
    MustProvideAddOrRemove,
}

impl From<ContractError> for NeutronError {
    fn from(val: ContractError) -> Self {
        NeutronError::Std(StdError::generic_err(val.to_string()))
    }
}
