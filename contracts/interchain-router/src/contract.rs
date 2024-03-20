use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::{
    neutron::{assert_ibc_fee_coverage, query_ibc_fee},
    soft_validate_remote_chain_addr,
};
use cw2::set_contract_version;
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    query::min_ibc_fee::MinIbcFeeResponse,
    NeutronError, NeutronResult,
};

use crate::state::{DESTINATION_CONFIG, TARGET_DENOMS};
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::CLOCK_ADDRESS,
};

type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;
type QueryDeps<'a> = Deps<'a, NeutronQuery>;

const CONTRACT_NAME: &str = "crates.io:covenant-interchain-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let clock_address = deps.api.addr_validate(&msg.clock_address)?;
    soft_validate_remote_chain_addr(deps.api, &msg.destination_config.destination_receiver_addr)?;

    CLOCK_ADDRESS.save(deps.storage, &clock_address)?;
    DESTINATION_CONFIG.save(deps.storage, &msg.destination_config)?;
    TARGET_DENOMS.save(deps.storage, &msg.denoms)?;

    Ok(Response::default()
        .add_message(enqueue_msg(msg.clock_address.as_str())?)
        .add_attribute("method", "interchain_router_instantiate")
        .add_attribute("clock_address", clock_address.to_string())
        .add_attributes(msg.destination_config.get_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::Tick {} => {
            // Verify caller is the clock
            verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;
            try_route_balances(deps, env)
        }
        ExecuteMsg::DistributeFallback { denoms } => {
            try_distribute_fallback(deps, env, info, denoms)
        }
    }
}

fn try_distribute_fallback(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    denoms: Vec<String>,
) -> NeutronResult<Response<NeutronMsg>> {
    let mut available_balances = Vec::with_capacity(denoms.len());
    let destination_config = DESTINATION_CONFIG.load(deps.storage)?;
    let explicit_denoms = TARGET_DENOMS.load(deps.storage)?;
    let min_ibc_fee_config = query_ibc_fee(deps.querier)?;

    assert_ibc_fee_coverage(
        info,
        min_ibc_fee_config.total_ntrn_fee,
        Uint128::from(denoms.len() as u128),
    )?;

    for denom in denoms {
        // we do not distribute the main covenant denoms
        // according to the fallback split
        if explicit_denoms.contains(&denom) {
            return Err(NeutronError::Std(StdError::generic_err(
                "unauthorized denom distribution",
            )));
        }
        let queried_coin = deps
            .querier
            .query_balance(env.contract.address.to_string(), denom)?;
        available_balances.push(queried_coin);
    }

    let fallback_distribution_messages = destination_config.get_ibc_transfer_messages_for_coins(
        available_balances,
        env.block.time,
        env.contract.address.to_string(),
        min_ibc_fee_config.ibc_fee,
    )?;

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback")
        .add_messages(fallback_distribution_messages))
}

/// method that attempts to transfer out all available balances to the receiver
fn try_route_balances(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let destination_config = DESTINATION_CONFIG.load(deps.storage)?;
    let denoms_to_route = TARGET_DENOMS.load(deps.storage)?;
    let mut denom_balances = Vec::with_capacity(denoms_to_route.len());

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

    let min_ibc_fee: MinIbcFeeResponse = deps.querier.query(&NeutronQuery::MinIbcFee {}.into())?;

    // get transfer messages for each denom
    let messages = destination_config.get_ibc_transfer_messages_for_coins(
        denom_balances,
        env.block.time,
        env.contract.address.to_string(),
        min_ibc_fee.min_fee,
    )?;

    Ok(Response::default()
        .add_attribute("method", "try_route_balances")
        .add_attributes(balance_attributes)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ReceiverConfig {} => {
            Ok(to_json_binary(&DESTINATION_CONFIG.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::TargetDenoms {} => Ok(to_json_binary(&TARGET_DENOMS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: ExecuteDeps,
    _env: Env,
    msg: MigrateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            destination_config,
            target_denoms,
        } => {
            let mut response =
                Response::default().add_attribute("method", "update_interchain_router");

            if let Some(addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("clock_addr", addr);
            }

            if let Some(denoms) = target_denoms {
                let denoms_str = denoms.join(",").to_string();
                let denom_set: BTreeSet<String> = denoms.into_iter().collect();
                TARGET_DENOMS.save(deps.storage, &denom_set)?;
                response = response.add_attribute("target_denoms", denoms_str);
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
