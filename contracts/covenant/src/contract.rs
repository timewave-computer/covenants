
use cosmwasm_std::{DepsMut, Deps, Env, Response, StdResult, MessageInfo, Binary, to_binary, SubMsg, CosmosMsg, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use crate::{msg::{QueryMsg, MigrateMsg, ExecuteMsg, InstantiateMsg}, error::ContractError, state::{LS_INSTANTIATION_DATA, CLOCK_INSTANTIATION_DATA, LP_INSTANTIATION_DATA, DEPOSITOR_INSTANTIATION_DATA}};

const CONTRACT_NAME: &str = "crates.io:covenant-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;


    CLOCK_INSTANTIATION_DATA.save(deps.storage, &(msg.clock_code, msg.clock_instantiate))?;
    LP_INSTANTIATION_DATA.save(deps.storage, &(msg.lp_code, msg.lp_instantiate))?;
    LS_INSTANTIATION_DATA.save(deps.storage, &(msg.ls_code, msg.ls_instantiate))?;
    DEPOSITOR_INSTANTIATION_DATA.save(deps.storage, &(msg.depositor_code, msg.depositor_instantiate))?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    to_binary(&true)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    Ok(Response::default())
}
