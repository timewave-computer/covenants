use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Empty, to_binary, Env, Deps, StdResult, Binary, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

use crate::msg::{SplitConfig, ReceiverType, NativeReceiver};

mod suite;
mod tests;


pub fn splitter_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

pub fn mock_protocol_guild_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        query,
    );
    Box::new(contract)
}


// timewave protocol guild mock
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SplitConfig)]
    PublicGoodsSplit {},
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PublicGoodsSplit {} => Ok(to_binary(&SplitConfig { 
            receivers: vec![
                (
                    ReceiverType::Native(NativeReceiver { address: "save_the_cats".to_string()}),
                    Uint128::new(100),
                ),
            ] 
        })?),
    }
}
