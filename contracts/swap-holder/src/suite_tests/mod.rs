use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, Deps, Empty, Env, StdResult};
use covenant_clock::test_helpers::helpers::{
    mock_clock_execute, mock_clock_instantiate, mock_clock_query,
};
use covenant_macros::covenant_deposit_address;
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;

pub fn swap_holder_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn mock_deposit_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        query,
    );
    Box::new(contract)
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&"native-splitter")?),
    }
}

pub fn mock_clock_deps_contract() -> Box<dyn Contract<Empty>> {
    let contract =
        ContractWrapper::new(mock_clock_execute, mock_clock_instantiate, mock_clock_query);

    Box::new(contract)
}
