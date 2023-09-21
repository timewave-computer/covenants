use astroport::asset::PairInfo;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, Deps, Empty, Env, StdResult, Addr};
use covenant_macros::covenant_deposit_address;
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;

pub fn two_party_pol_holder_contract() -> Box<dyn Contract<Empty>> {
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
        QueryMsg::DepositAddress {} => Ok(to_binary(&"splitter")?),
    }
}


pub fn mock_astro_pool_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        query_astro_pool,
    );
    Box::new(contract)
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query_astro_pool(_deps: Deps, _env: Env, msg: astroport::pair::QueryMsg) -> StdResult<Binary> {
    match msg {
        astroport::pair::QueryMsg::Pair {} => Ok(to_binary(&PairInfo {
            asset_infos: vec![],
            contract_addr: Addr::unchecked("lp-token"),
            liquidity_token: Addr::unchecked("lp-token"),
            pair_type: astroport::factory::PairType::Xyk {  },
        })?),
        _ => Ok(to_binary(&"-")?),
    }
}
