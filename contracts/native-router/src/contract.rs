use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, Attribute, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use valence_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::migrate_helper::get_recover_msg;
use cw2::{get_contract_version, set_contract_version};
use neutron_sdk::NeutronError;
use semver::Version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{CLOCK_ADDRESS, RECEIVER_ADDRESS, TARGET_DENOMS},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
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
    let receiver_addr = deps.api.addr_validate(&msg.receiver_address)?;

    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    RECEIVER_ADDRESS.save(deps.storage, &receiver_addr)?;
    TARGET_DENOMS.save(deps.storage, &msg.denoms)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "interchain_router_instantiate")
        .add_attribute("clock_address", clock_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => {
            // Verify caller is the clock
            verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;
            try_route_balances(deps, env)
        }
        ExecuteMsg::DistributeFallback { denoms } => try_distribute_fallback(deps, env, denoms),
        ExecuteMsg::RecoverFunds { denoms } => {
            let covenant_addr = deps.querier.query_wasm_contract_info(
                env.contract.address.as_str()
            )?
            .creator;

            let holder_addr = if let Some(resp) = deps.querier.query_wasm_raw(
                covenant_addr,
                b"covenant_two_party_pol_holder_addr".as_slice(),
            )? {
                let resp: Addr = from_json(resp)?;
                resp
            } else {
                return Err(ContractError::Std(StdError::generic_err("holder address not found")))
            };

            // query the holder for emergency commitee address
            let commitee_raw_query = deps.querier.query_wasm_raw(
                holder_addr.to_string(),
                b"e_c_a".as_slice(),
            )?;
            let emergency_commitee: Addr = if let Some(resp) = commitee_raw_query {
                let resp: Addr = from_json(resp)?;
                resp
            } else {
                return Err(ContractError::Std(StdError::generic_err("emergency committee address not found")))
            };

            // validate emergency committee as caller
            ensure!(
                info.sender == emergency_commitee,
                ContractError::Std(StdError::generic_err("only emergency committee can recover funds"))
            );

            // collect available denom coins into a bank send
            let recover_msg = get_recover_msg(deps, env, denoms, emergency_commitee.to_string())?;
            Ok(Response::new()
                .add_message(recover_msg)
            )
        },
    }
}

fn try_distribute_fallback(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut available_balances = Vec::with_capacity(denoms.len());
    let receiver_address = RECEIVER_ADDRESS.load(deps.storage)?;
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

    let bank_sends: Vec<CosmosMsg> = available_balances
        .into_iter()
        .map(|c| {
            BankMsg::Send {
                to_address: receiver_address.to_string(),
                amount: vec![c],
            }
            .into()
        })
        .collect();

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback")
        .add_messages(bank_sends))
}

/// method that attempts to transfer out all available balances to the receiver
fn try_route_balances(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let receiver_addr = RECEIVER_ADDRESS.load(deps.storage)?;
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

    let bank_sends: Vec<CosmosMsg> = denom_balances
        .into_iter()
        .map(|c| {
            BankMsg::Send {
                to_address: receiver_addr.to_string(),
                amount: vec![c],
            }
            .into()
        })
        .collect();

    Ok(Response::default()
        .add_attribute("method", "try_route_balances")
        .add_attributes(balance_attributes)
        .add_messages(bank_sends))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ReceiverConfig {} => {
            Ok(to_json_binary(&RECEIVER_ADDRESS.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::TargetDenoms {} => Ok(to_json_binary(&TARGET_DENOMS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            receiver_address,
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

            if let Some(addr) = receiver_address {
                RECEIVER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("receiver_addr", addr);
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            let version: Version = match CONTRACT_VERSION.parse() {
                Ok(v) => v,
                Err(e) => return Err(ContractError::NeutronError(NeutronError::Std(StdError::generic_err(e.to_string())))),
            };

            let storage_version: Version = match get_contract_version(deps.storage)?.version.parse() {
                Ok(v) => v,
                Err(e) => return Err(ContractError::NeutronError(NeutronError::Std(StdError::generic_err(e.to_string())))),
            };
            if storage_version < version {
                set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            }
            Ok(Response::new())
        }
    }
}
