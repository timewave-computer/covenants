#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    DepsMut, Env, MessageInfo,
    Response, Order, CosmosMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ExecuteMsg};
use crate::state::SPLIT_CONFIG_MAP;

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

    // we validate the splits and store them per-denom
    for (denom, split) in msg.splits {
        let validated_split = split.validate_to_split_config()?;
        SPLIT_CONFIG_MAP.save(deps.storage, denom, &validated_split)?;
    }

    Ok(Response::default()
        .add_attribute("method", "interchain_splitter_instantiate")
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
    match msg {
        ExecuteMsg::Tick {} => try_distribute(deps, env, info),
    }
}

pub fn try_distribute(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // first we query the contract balances
    let balances = deps.querier.query_all_balances(env.contract.address)?;
    let mut distribution_messages: Vec<CosmosMsg> = vec![];
    // then we iterate over our split config and try to match the entries to available balances
    for entry in SPLIT_CONFIG_MAP.range(deps.storage, None, None, Order::Ascending) {
        let (denom, config) = entry?;

        // if we have the denom in our balances we construct the split messages
        if let Some(coin) = balances.iter().find(|c| c.denom == denom) {
            let mut transfer_messages = config.get_transfer_messages(coin.amount, coin.denom.to_string())?;
            distribution_messages.append(&mut transfer_messages);
        }
    }
    
    Ok(Response::default()
        .add_messages(distribution_messages)
    )
}