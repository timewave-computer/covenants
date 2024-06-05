use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Attribute, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult,
};
use covenant_utils::op_mode::{verify_caller, ContractOperationMode};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{CONTRACT_OP_MODE, RECEIVER_ADDRESS, TARGET_DENOMS},
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

    let op_mode = ContractOperationMode::try_init(deps.api, msg.op_mode_cfg.clone())?;
    let receiver_addr = deps.api.addr_validate(&msg.receiver_address)?;

    CONTRACT_OP_MODE.save(deps.storage, &op_mode)?;
    RECEIVER_ADDRESS.save(deps.storage, &receiver_addr)?;
    TARGET_DENOMS.save(deps.storage, &msg.denoms)?;

    Ok(Response::default()
        .add_attribute("method", "interchain_router_instantiate")
        .add_attribute("op_mode", format!("{:?}", op_mode)))
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
            verify_caller(&info.sender, &CONTRACT_OP_MODE.load(deps.storage)?)?;
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
        QueryMsg::TargetDenoms {} => Ok(to_json_binary(&TARGET_DENOMS.may_load(deps.storage)?)?),
        QueryMsg::OperationMode {} => {
            Ok(to_json_binary(&CONTRACT_OP_MODE.may_load(deps.storage)?)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::UpdateConfig {
            op_mode,
            receiver_address,
            target_denoms,
        } => {
            let mut response =
                Response::default().add_attribute("method", "update_interchain_router");

            if let Some(op_mode_cfg) = op_mode {
                let updated_op_mode = ContractOperationMode::try_init(deps.api, op_mode_cfg)
                    .map_err(|err| StdError::generic_err(err.to_string()))?;

                CONTRACT_OP_MODE.save(deps.storage, &updated_op_mode)?;
                response = response.add_attribute("op_mode", format!("{:?}", updated_op_mode));
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
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default().add_attribute("method", "update_native_router"))
        }
    }
}
