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
}

impl ContractError {
    pub fn to_neutron_std(&self) -> NeutronError {
        NeutronError::Std(StdError::generic_err(self.to_string()))
    }
}
