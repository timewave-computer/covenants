use cosmwasm_std::StdError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
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

    #[error("Caller is not the clock, only clock can tick contracts")]
    NotClock,
}

impl Into<NeutronError> for ContractError {
    fn into(self) -> NeutronError {
        NeutronError::Std(StdError::generic_err(self.to_string()))
    }
}
