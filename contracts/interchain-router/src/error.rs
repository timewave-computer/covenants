use cosmwasm_std::StdError;
use covenant_utils::op_mode::ContractOperationError;
use cw_utils::PaymentError;
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ContractOperationError(#[from] ContractOperationError),

    #[error("unauthorized to distribute explicitly defined denom")]
    UnauthorizedDenomDistribution {},

    #[error("caller must cover ibc fees: {0}")]
    IbcFeeError(PaymentError),
}

impl ContractError {
    pub fn to_neutron_std(&self) -> NeutronError {
        NeutronError::Std(StdError::generic_err(self.to_string()))
    }
}
