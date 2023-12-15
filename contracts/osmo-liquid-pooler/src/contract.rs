#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, ProvidedLiquidityInfo, QueryMsg},
    state::{HOLDER_ADDRESS, PROVIDED_LIQUIDITY_INFO},
};

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate the contract addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let _pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant module addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;

    // we begin with no liquidity provided
    PROVIDED_LIQUIDITY_INFO.save(
        deps.storage,
        &ProvidedLiquidityInfo {
            provided_amount_a: Uint128::zero(),
            provided_amount_b: Uint128::zero(),
        },
    )?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "lp_instantiate")
        .add_attribute("clock_addr", clock_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_lp(deps, env),
    }
}

fn try_lp(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    Ok(Response::default().add_attribute("method", "try_lp"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ProvidedLiquidityInfo {} => Ok(to_json_binary(
            &PROVIDED_LIQUIDITY_INFO.load(deps.storage)?,
        )?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
    }
}
