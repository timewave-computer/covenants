use cosmwasm_std::StdError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Next contract is not ready for receiving the funds yet")]
    DepositAddressNotAvailable {},

    #[error("Missing fallback address")]
    MissingFallbackAddress {},

    #[error("Cannot distribute target denom via fallback distribution")]
    UnauthorizedDenomDistribution {},

    #[error("Attempt to distribute duplicate denoms via fallback distribution")]
    DuplicateDenomDistribution {},
}

impl From<ContractError> for NeutronError {
    fn from(value: ContractError) -> Self {
        NeutronError::Std(StdError::generic_err(value.to_string()))
    }
}
