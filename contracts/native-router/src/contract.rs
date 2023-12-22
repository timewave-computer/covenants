use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::{get_default_ibc_fee_requirement, ReceiverConfig};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{RECEIVER_CONFIG, TARGET_DENOMS},
};
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::CLOCK_ADDRESS,
};

const CONTRACT_NAME: &str = "crates.io:covenant-native-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    CLOCK_ADDRESS.save(deps.storage, &Addr::unchecked(msg.clock_address))?;
    RECEIVER_CONFIG.save(deps.storage, &msg.receiver_config)?;
    TARGET_DENOMS.save(deps.storage, &msg.denoms)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "interchain_router_instantiate")
        .add_attribute("clock_address", clock_addr))
    // .add_attributes(destination_config.get_response_attributes()))
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
    match msg {
        ExecuteMsg::Tick {} => {
            // Verify caller is the clock
            verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;
            try_route_balances(deps, env)
        }
        ExecuteMsg::DistributeFallback { denoms } => try_distribute_fallback(deps, env, denoms),
    }
}

fn try_distribute_fallback(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut available_balances = Vec::new();
    let receiver_config = RECEIVER_CONFIG.load(deps.storage)?;
    let explicit_denoms = TARGET_DENOMS.load(deps.storage)?;

    for denom in denoms.clone() {
        // we do not distribute the main covenant denoms
        // according to the fallback split
        if explicit_denoms.contains(&denom) {
            return Err(ContractError::Std(StdError::generic_err(
                "unauthorized denom distribution",
            )));
        }
        let queried_coin = deps
            .querier
            .query_balance(env.contract.address.to_string(), denom)?;
        available_balances.push(queried_coin);
    }

    let messages: Vec<CosmosMsg> = match receiver_config {
        ReceiverConfig::Native(addr) => {
            let mut bank_sends: Vec<CosmosMsg> = vec![];
            // we get the number of target denoms we have to reserve
            // neutron fees for
            let count = Uint128::from(denoms.len() as u128);

            for coin in available_balances {
                let send_coin = if coin.denom != "untrn" {
                    Some(coin)
                } else {
                    // if its neutron we're distributing we need to keep a
                    // reserve for ibc gas costs.
                    // this is safe because we pass target denoms.
                    let reserve_amount = count * get_default_ibc_fee_requirement();
                    if coin.amount > reserve_amount {
                        Some(Coin {
                            denom: coin.denom,
                            amount: coin.amount - reserve_amount,
                        })
                    } else {
                        None
                    }
                };

                match send_coin {
                    Some(c) => bank_sends.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                        to_address: addr.to_string(),
                        amount: vec![c],
                    })),
                    None => (),
                }
            }
            bank_sends
        }
        ReceiverConfig::Ibc(_destination_config) => vec![],
    };

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback")
        .add_messages(messages))
}

/// method that attempts to transfer out all available balances to the receiver
fn try_route_balances(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let receiver_config = RECEIVER_CONFIG.load(deps.storage)?;
    let denoms_to_route = TARGET_DENOMS.load(deps.storage)?;
    let mut denom_balances = Vec::new();
    for denom in denoms_to_route {
        let coin_to_route = deps
            .querier
            .query_balance(env.contract.address.to_string(), denom)?;
        if !coin_to_route.amount.is_zero() {
            denom_balances.push(coin_to_route);
        }
    }

    // if there are no balances, we return early;
    // otherwise build up the response attributes
    let balance_attributes: Vec<Attribute> = match denom_balances.len() {
        0 => {
            return Ok(Response::default()
                .add_attribute("method", "try_route_balances")
                .add_attribute("balances", "[]"))
        }
        1 => vec![Attribute::new(
            denom_balances[0].denom.to_string(),
            denom_balances[0].amount,
        )],
        _ => denom_balances
            .iter()
            .map(|c| Attribute::new(c.denom.to_string(), c.amount))
            .collect(),
    };

    // get transfer messages for each denom
    let messages: Vec<CosmosMsg> = match receiver_config {
        covenant_utils::ReceiverConfig::Native(addr) => {
            let mut bank_sends: Vec<CosmosMsg> = vec![];
            // we get the number of target denoms we have to reserve
            // neutron fees for
            let count = Uint128::from(1 + denom_balances.len() as u128);

            for coin in denom_balances {
                // non-neutron coins get distributed entirely
                let send_coin = if coin.denom != "untrn" {
                    Some(coin)
                } else {
                    // if its neutron we're distributing we need to keep a
                    // reserve for ibc gas costs.
                    // this is safe because we pass target denoms.
                    let reserve_amount = count * get_default_ibc_fee_requirement();
                    if coin.amount > reserve_amount {
                        Some(Coin {
                            denom: coin.denom,
                            amount: coin.amount - reserve_amount,
                        })
                    } else {
                        None
                    }
                };

                match send_coin {
                    Some(c) => bank_sends.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                        to_address: addr.to_string(),
                        amount: vec![c],
                    })),
                    None => (),
                }
            }
            bank_sends
        }
        covenant_utils::ReceiverConfig::Ibc(_destination_config) => vec![],
    };

    Ok(Response::default()
        .add_attribute("method", "try_route_balances")
        .add_attributes(balance_attributes)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ReceiverConfig {} => {
            Ok(to_json_binary(&RECEIVER_CONFIG.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::TargetDenoms {} => Ok(to_json_binary(&TARGET_DENOMS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            receiver_config,
            target_denoms,
        } => {
            let mut response =
                Response::default().add_attribute("method", "update_interchain_router");

            if let Some(addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("clock_addr", addr);
            }

            if let Some(denoms) = target_denoms {
                let denoms_str = denoms.join(",");
                let denom_set: BTreeSet<String> = denoms.into_iter().collect();
                TARGET_DENOMS.save(deps.storage, &denom_set)?;
                response = response.add_attribute("target_denoms", denoms_str);
            }

            if let Some(config) = receiver_config {
                RECEIVER_CONFIG.save(deps.storage, &config)?;
                // response = response.add_attributes(config.get_response_attributes());
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
