use std::collections::HashSet;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, StdError};
use neutron_sdk::NeutronError;
use thiserror::Error;

#[cw_serde]
pub enum ContractOperationModeConfig {
    Permissionless,
    Permissioned(Vec<String>),
}

#[cw_serde]
pub enum ContractOperationMode {
    Permissionless,
    Permissioned(PrivilegedAccounts),
}

#[derive(Error, Debug, PartialEq)]
pub enum ContractOperationError {
    #[error("Contract operation unauthorized")]
    Unauthorized,
}

impl From<ContractOperationError> for NeutronError {
    fn from(op_err: ContractOperationError) -> Self {
        NeutronError::Std(StdError::generic_err(op_err.to_string()))
    }
}

#[cw_serde]
pub struct PrivilegedAccounts(HashSet<Addr>);

impl ContractOperationMode {
    pub fn try_init(
        api: &dyn Api,
        op_mode_cfg: ContractOperationModeConfig,
    ) -> Result<Self, StdError> {
        match op_mode_cfg {
            ContractOperationModeConfig::Permissionless => {
                Ok(ContractOperationMode::Permissionless)
            }
            ContractOperationModeConfig::Permissioned(addresses) => {
                ensure!(
                    !addresses.is_empty(),
                    StdError::generic_err("privileged_accounts cannot be empty")
                );

                let privileged_accounts = addresses
                    .iter()
                    .map(|addr| api.addr_validate(addr))
                    .collect::<Result<HashSet<_>, StdError>>()?;

                Ok(ContractOperationMode::Permissioned(
                    PrivilegedAccounts::new(privileged_accounts),
                ))
            }
        }
    }
}

impl PrivilegedAccounts {
    pub fn new(privileged_accounts: HashSet<Addr>) -> Self {
        assert!(
            !privileged_accounts.is_empty(),
            "privileged_accounts cannot be empty"
        );
        Self(privileged_accounts)
    }

    pub fn is_privileged(&self, addr: &Addr) -> bool {
        self.0.contains(addr)
    }
}

impl From<HashSet<Addr>> for PrivilegedAccounts {
    fn from(privileged_accounts: HashSet<Addr>) -> Self {
        Self::new(privileged_accounts)
    }
}

impl From<Vec<Addr>> for PrivilegedAccounts {
    fn from(privileged_accounts: Vec<Addr>) -> Self {
        Self(privileged_accounts.into_iter().collect())
    }
}

pub fn verify_caller(
    caller: &Addr,
    op_mode: &ContractOperationMode,
) -> Result<(), ContractOperationError> {
    if let ContractOperationMode::Permissioned(privileged_accounts) = op_mode {
        if !privileged_accounts.is_privileged(caller) {
            return Err(ContractOperationError::Unauthorized);
        }
    }
    Ok(())
}
