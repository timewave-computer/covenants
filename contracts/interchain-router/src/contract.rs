#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Env, MessageInfo, Response, DepsMut, Attribute};
use cw2::set_contract_version;

use crate::{msg::{InstantiateMsg, ExecuteMsg, DestinationConfig}, state::{CLOCK_ADDRESS, DESTINATION_CONFIG}, error::ContractError};


const CONTRACT_NAME: &str = "crates.io:covenant-interchain-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let destination_receiver_addr = deps.api.addr_validate(&msg.destination_receiver_addr)?;

    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    DESTINATION_CONFIG.save(deps.storage, &DestinationConfig {
        destination_chain_channel_id: msg.destination_chain_channel_id.to_string(),
        destination_receiver_addr,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
    })?;

    Ok(Response::default()
        .add_attribute("method", "interchain_router_instantiate")
        .add_attributes(msg.get_response_attributes())
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {msg:?}").as_str());

    // Verify caller is the clock
    if info.sender != CLOCK_ADDRESS.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    match msg {
        ExecuteMsg::Tick {} => try_route_balances(deps, env),
    }
}

fn try_route_balances(deps: DepsMut, env: Env) -> Result<Response, ContractError> {

    let balances = deps.querier.query_all_balances(env.contract.address)?;
    let destination_config: DestinationConfig = DESTINATION_CONFIG.load(deps.storage)?;

    let balance_attributes: Vec<Attribute> = balances.iter()
        .map(|c| Attribute::new(c.denom.to_string(), c.amount))
        .collect();

    let messages = destination_config.get_ibc_transfer_messages_for_coins(balances, env.block.time);

    Ok(Response::default()
        .add_attribute("method", "try_route_balances")
        .add_attributes(balance_attributes)
        .add_messages(messages)
    )
}