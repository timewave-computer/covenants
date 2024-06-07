use cosmwasm_std::StdError;
use covenant_utils::op_mode::ContractOperationError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    NeutronError(#[from] NeutronError),

    #[error(transparent)]
    ContractOperationError(#[from] ContractOperationError),

    #[error("unauthorized to distribute explicitly defined denom")]
    UnauthorizedDenomDistribution {},
}
