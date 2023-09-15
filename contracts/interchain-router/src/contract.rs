#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use covenant_utils::DestinationConfig;
use cw2::set_contract_version;
use neutron_sdk::bindings::msg::NeutronMsg;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{CLOCK_ADDRESS, DESTINATION_CONFIG},
};

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

    let clock_addr  = deps.api.addr_validate(&msg.clock_address)?;

    let destination_config = DestinationConfig {
        destination_chain_channel_id: msg.destination_chain_channel_id.to_string(),
        destination_receiver_addr: msg.destination_receiver_addr.to_string(),
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
    };

    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    DESTINATION_CONFIG.save(deps.storage, &destination_config)?;

    Ok(Response::default()
        .add_attribute("method", "interchain_router_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attribute("destination_receiver_addr", msg.destination_receiver_addr)
        .add_attributes(destination_config.get_response_attributes())
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {

    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {msg:?}").as_str());

    // Verify caller is the clock
    // if info.sender != CLOCK_ADDRESS.load(deps.storage)? {
    //     return Err(ContractError::Unauthorized {});
    // }

    match msg {
        ExecuteMsg::Tick {} => try_route_balances(deps, env),
    }
}

/// method that attempts to transfer out all available balances to the receiver
fn try_route_balances(deps: DepsMut, env: Env) -> Result<Response<NeutronMsg>, ContractError> {

    let destination_config: DestinationConfig = DESTINATION_CONFIG.load(deps.storage)?;

    // first we query all balances of the router
    let balances = deps.querier.query_all_balances(env.clone().contract.address)?;

    // if there are no balances, we return early;
    // otherwise build up the response attributes
    let balance_attributes: Vec<Attribute> = if balances.is_empty() {
        return Ok(Response::default()
            .add_attribute("method", "try_route_balances")
            .add_attribute("balances", "[]"));
    } else {
        balances
            .iter()
            .map(|c| Attribute::new(c.denom.to_string(), c.amount))
            .collect()
    };

    // get ibc transfer messages for each denom
    let messages = destination_config.get_ibc_transfer_messages_for_coins(
        balances,
        env.clone().block.time,
        env.contract.address.to_string(),
    );
    
    Ok(Response::default()
        .add_attribute("method", "try_route_balances")
        .add_attributes(balance_attributes)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DestinationConfig {} => {
            Ok(to_binary(&DESTINATION_CONFIG.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            destination_config,
        } => {
            let mut response =
                Response::default().add_attribute("method", "update_interchain_router");

            if let Some(addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("clock_addr", addr);
            }

            if let Some(config) = destination_config {
                DESTINATION_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.get_response_attributes());
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default().add_attribute("method", "update_interchain_router"))
        }
    }
}
