use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::migrate_helper::get_recover_msg;
use covenant_utils::split::SplitConfig;
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{CLOCK_ADDRESS, FALLBACK_SPLIT, SPLIT_CONFIG_MAP};

const CONTRACT_NAME: &str = "crates.io:covenant-native-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let mut resp = Response::default().add_attribute("method", "native_splitter_instantiate");

    let clock_address = deps.api.addr_validate(&msg.clock_address)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_address)?;
    resp = resp.add_attribute("clock_addr", msg.clock_address.to_string());

    // we validate the splits and store them per-denom
    for (denom, split) in msg.splits {
        split.validate_shares_and_receiver_addresses(deps.api)?;
        SPLIT_CONFIG_MAP.save(deps.storage, denom.to_string(), &split)?;
    }

    // if a fallback split is provided we validate and store it
    if let Some(split) = msg.fallback_split {
        resp = resp.add_attributes(vec![split.get_response_attribute("fallback".to_string())]);
        split.validate_shares_and_receiver_addresses(deps.api)?;
        FALLBACK_SPLIT.save(deps.storage, &split)?;
    } else {
        resp = resp.add_attribute("fallback", "None");
    }

    Ok(resp
        .add_message(enqueue_msg(msg.clock_address.as_str())?)
        .add_attribute("clock_address", clock_address))
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
            verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)
                .map_err(|_| ContractError::NotClock)?;

            try_distribute(deps, env)
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

pub fn try_distribute(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // first we query the contract balances
    let mut distribution_messages: Vec<CosmosMsg> = vec![];

    // then we iterate over our split config and try to match the entries to available balances
    for entry in SPLIT_CONFIG_MAP.range(deps.storage, None, None, Order::Ascending) {
        let (denom, config) = entry?;
        let balance = deps
            .querier
            .query_balance(env.contract.address.clone(), denom.to_string())?;

        if !balance.amount.is_zero() {
            let mut transfer_messages =
                config.get_transfer_messages(balance.amount, balance.denom.to_string(), None)?;
            distribution_messages.append(&mut transfer_messages);
        }
    }

    Ok(Response::default()
        .add_attribute("method", "try_distribute")
        .add_messages(distribution_messages))
}

fn try_distribute_fallback(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut distribution_messages: Vec<CosmosMsg> = vec![];

    if let Some(split) = FALLBACK_SPLIT.may_load(deps.storage)? {
        for denom in denoms {
            // we do not distribute the main covenant denoms
            // according to the fallback split
            ensure!(
                !SPLIT_CONFIG_MAP.has(deps.storage, denom.to_string()),
                ContractError::Std(StdError::generic_err("unauthorized denom distribution"))
            );

            let balance = deps
                .querier
                .query_balance(env.contract.address.to_string(), denom)?;
            if !balance.amount.is_zero() {
                let mut fallback_messages =
                    split.get_transfer_messages(balance.amount, balance.denom, None)?;
                distribution_messages.append(&mut fallback_messages);
            }
        }
    } else {
        return Err(StdError::generic_err("no fallback split defined").into());
    }

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback")
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

    Ok(SplitConfig {
        receivers: BTreeMap::new(),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, StdError> {
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
                for (denom, split) in splits {
                    // we validate each split before storing it
                    SPLIT_CONFIG_MAP.save(deps.storage, denom.to_string(), &split)?;
                }
            }

            if let Some(split) = fallback_split {
                FALLBACK_SPLIT.save(deps.storage, &split)?;
                resp =
                    resp.add_attributes(vec![split.get_response_attribute("fallback".to_string())]);
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            let version: Version = match CONTRACT_VERSION.parse() {
                Ok(v) => v,
                Err(e) => return Err(StdError::generic_err(e.to_string())),
            };

            let storage_version: Version = match get_contract_version(deps.storage)?.version.parse() {
                Ok(v) => v,
                Err(e) => return Err(StdError::generic_err(e.to_string())),
            };
            if storage_version < version {
                set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            }
            Ok(Response::new())
        }
    }
}
