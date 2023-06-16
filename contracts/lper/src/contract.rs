#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{MessageInfo,  Response,
     StdResult, Addr, DepsMut, Env, Binary, Deps, to_binary, 
};
use cw2::set_contract_version;


use crate::{msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg}, state::{HOLDER_ADDRESS, LP_POSITION}};

use neutron_sdk::{
    bindings::{
        msg::{NeutronMsg},
        query::{NeutronQuery},
    },
    NeutronResult,
};

use crate::state::{
   CLOCK_ADDRESS, CONTRACT_STATE, ContractState,
};


const CONTRACT_NAME: &str = "crates.io:stride-lper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: validations
    CLOCK_ADDRESS.save(deps.storage, &Addr::unchecked(msg.clock_address))?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    LP_POSITION.save(deps.storage, &msg.lp_position)?;
    HOLDER_ADDRESS.save(deps.storage, &msg.holder_address)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::WithdrawRewards {} => try_withdraw(deps, env, info),
    }
}


fn try_tick(mut deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    let current_state = CONTRACT_STATE.load(deps.storage)?;

    match current_state {
        ContractState::Instantiated => try_enter_lp_position(deps, env, info),
        ContractState::LpPositionEntered => no_op(),
        ContractState::LpPositionExited => no_op(),
        ContractState::WithdrawComplete => no_op(),
    }
}

fn no_op() -> NeutronResult<Response<NeutronMsg>> {
    Ok(Response::default())
}

fn try_enter_lp_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo, 
) -> NeutronResult<Response<NeutronMsg>> {
    Ok(Response::default())

}

fn try_withdraw(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo, 
) -> NeutronResult<Response<NeutronMsg>> {
    Ok(Response::default())
    
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        
        QueryMsg::ClockAddress {} => Ok(
            to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?
        ),
        QueryMsg::LPPosition {} => Ok(
            to_binary(&LP_POSITION.may_load(deps.storage)?)?
        )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    Ok(Response::default())
}
