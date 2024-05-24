use std::collections::HashSet;

use cosmwasm_std::{ensure, Addr, Api, StdError};

pub fn validate_privileged_accounts(
    api: &dyn Api,
    privileged_accounts: Option<Vec<String>>,
) -> Result<Option<HashSet<Addr>>, StdError> {
    privileged_accounts
        .map(|addresses| {
            ensure!(
                !addresses.is_empty(),
                StdError::generic_err("privileged_accounts cannot be empty")
            );

            addresses
                .iter()
                .map(|addr| api.addr_validate(addr))
                .collect::<Result<HashSet<_>, StdError>>()
        })
        .transpose()
}

pub fn verify_caller(
    caller: &Addr,
    privileged_accounts: &Option<HashSet<Addr>>,
) -> Result<(), StdError> {
    if let Some(privileged_accounts) = privileged_accounts {
        if !privileged_accounts.contains(caller) {
            return Err(StdError::generic_err("Unauthorized"));
        }
    }
    Ok(())
}
