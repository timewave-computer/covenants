use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use covenant_utils::SplitConfig;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SplitType};
use crate::state::{CLOCK_ADDRESS, FALLBACK_SPLIT, SPLIT_CONFIG_MAP};

const CONTRACT_NAME: &str = "crates.io:covenant-interchain-splitter";
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

    let mut resp = Response::default().add_attribute("method", "interchain_splitter_instantiate");

    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    resp = resp.add_attribute("clock_addr", clock_addr.to_string());

    // we validate the splits and store them per-denom
    for (denom, split) in msg.splits {
        // split.get_split_config()?.validate()?;
        match split {
            SplitType::Custom(config) => {
                SPLIT_CONFIG_MAP.save(deps.storage, denom.to_string(), &config)?;
            }
        }

        // resp = resp.add_attributes(vec![split.get_response_attribute(denom)]);
    }

    // if a fallback split is provided we validate and store it
    if let Some(split) = msg.fallback_split {
        resp = resp.add_attributes(vec![split
            .clone()
            .get_response_attribute("fallback".to_string())]);
        FALLBACK_SPLIT.save(deps.storage, &split)?;
    } else {
        resp = resp.add_attribute("fallback", "None");
    }

    Ok(resp.add_message(enqueue_msg(clock_addr.as_str())?))
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
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::Tick {} => try_distribute(deps, env),
    }
}

pub fn try_distribute(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // first we query the contract balances
    let mut balances = deps.querier.query_all_balances(env.contract.address)?;
    let mut distribution_messages: Vec<CosmosMsg> = vec![];

    // then we iterate over our split config and try to match the entries to available balances
    for entry in SPLIT_CONFIG_MAP.range(deps.storage, None, None, Order::Ascending) {
        let (denom, config) = entry?;

        // we try to find the index of matching coin in available balances
        let balances_index = balances.iter().position(|coin| coin.denom == denom);
        if let Some(index) = balances_index {
            // pop the relevant coin and build the transfer messages
            let coin = balances.remove(index);
            let mut transfer_messages =
                config.get_transfer_messages(coin.amount, coin.denom.to_string(), None)?;
            distribution_messages.append(&mut transfer_messages);
        }
    }

    // by now all explicitly defined denom splits have been removed from the
    // balances vector so we can take the remaining balances and distribute
    // them according to the fallback split (if provided)
    if let Some(split) = FALLBACK_SPLIT.may_load(deps.storage)? {
        // get the distribution messages and add them to the list
        for leftover_bal in balances {
            let mut fallback_messages =
                split.get_transfer_messages(leftover_bal.amount, leftover_bal.denom, None)?;
            distribution_messages.append(&mut fallback_messages);
        }
    }

    Ok(Response::default()
        .add_attribute("method", "try_distribute")
        .add_messages(distribution_messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::DenomSplit { denom } => Ok(to_json_binary(&query_split(deps, denom)?)?),
        QueryMsg::Splits {} => Ok(to_json_binary(&query_all_splits(deps)?)?),
        QueryMsg::FallbackSplit {} => Ok(to_json_binary(&FALLBACK_SPLIT.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&Some(env.contract.address))?),
    }
}

pub fn query_all_splits(deps: Deps) -> Result<Vec<(String, SplitConfig)>, StdError> {
    let mut splits: Vec<(String, SplitConfig)> = vec![];

    for entry in SPLIT_CONFIG_MAP.range(deps.storage, None, None, Order::Ascending) {
        let (denom, config) = entry?;
        splits.push((denom, config));
    }

    Ok(splits)
}

pub fn query_split(deps: Deps, denom: String) -> Result<SplitConfig, StdError> {
    for entry in SPLIT_CONFIG_MAP.range(deps.storage, None, None, Order::Ascending) {
        let (entry_denom, config) = entry?;
        if entry_denom == denom {
            return Ok(config);
        }
    }

    Ok(SplitConfig { receivers: BTreeMap::new() })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            splits,
            fallback_split,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
                resp = resp.add_attribute("clock_addr", clock_addr);
            }

            if let Some(splits) = splits {
                // clear all current split configs before storing new values
                SPLIT_CONFIG_MAP.clear(deps.storage);
                for (denom, split_type) in splits {
                    match split_type {
                        // we validate each split before storing it
                        SplitType::Custom(split) => {
                            SPLIT_CONFIG_MAP.save(deps.storage, denom.to_string(), &split)?;
                        }
                    }
                }
            }

            if let Some(split) = fallback_split {

                FALLBACK_SPLIT.save(deps.storage, &split)?;
                resp = resp.add_attributes(vec![
                    split.get_response_attribute("fallback".to_string())
                ]);
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
