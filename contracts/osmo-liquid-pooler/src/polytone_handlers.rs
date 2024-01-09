use std::str::FromStr;

use cosmwasm_std::{from_json, Binary, Coin, DepsMut, Response, Uint128};
use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;
use polytone::callbacks::{ErrorResponse, ExecutionResponse};

use crate::{
    msg::ContractState,
    state::{CALLBACKS, CONTRACT_STATE, LIQUIDITY_PROVISIONING_CONFIG},
    contract::{PROXY_BALANCES_CALLBACK_ID, LIQUIDITY_PROVIDED_CALLBACK_ID, PROXY_CREATED_CALLBACK_ID},
};

// TODO: clean this up
pub fn process_query_callback(
    deps: DepsMut,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    // either query_pool or proxy_balances
    let initiator_msg: u8 = from_json(initiator_msg)?;
    let mut coin_balances = vec![];

    let entries = match query_callback_result {
        Ok(response) => {
            let mut responses = vec![];
            for resp in response {
                if initiator_msg == PROXY_BALANCES_CALLBACK_ID {
                    let res: QueryBalanceResponse = from_json(resp.clone())?;
                    if let Some(balance) = res.balance {
                        let cw_coin = Coin {
                            denom: balance.denom,
                            amount: Uint128::from_str(&balance.amount)?,
                        };
                        coin_balances.push(cw_coin);
                    }
                }
                responses.push(resp.to_string());
            }
            responses
        }
        Err(err) => vec![format!("{:?} : {:?}", err.message_index, err.error)],
    };

    if !coin_balances.is_empty() {
        let mut denom_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
        coin_balances.iter().for_each(|c| {
            denom_config
                .latest_balances
                .insert(c.denom.to_string(), c.clone());
        });

        LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &denom_config)?;
    }

    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
    callbacks.push(initiator_msg.to_string());
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

pub fn process_execute_callback(
    deps: DepsMut,
    execute_callback_result: Result<ExecutionResponse, String>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    let mut entries: Vec<String> = vec![];
    let initiator_msg: u8 = from_json(initiator_msg)?;
    match execute_callback_result {
        Ok(execution_response) => {
            for result in execution_response.result {
                let decoded = match result.data {
                    Some(data) => {
                        if initiator_msg == LIQUIDITY_PROVIDED_CALLBACK_ID {
                            entries.push("liquidity_provided".to_string());
                        } else if initiator_msg == PROXY_CREATED_CALLBACK_ID {
                            entries.push("proxy_created".to_string());
                        }
                        data.to_string()
                    }
                    None => "none".to_string(),
                };
                entries.push(decoded);
            }
        }
        Err(str) => entries.push(str),
    };

    if entries.contains(&"liquidity_provided".to_string()) {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
    }
    if entries.contains(&"proxy_created".to_string()) {
        CONTRACT_STATE.save(deps.storage, &ContractState::ProxyCreated)?;
    }
    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

pub fn process_fatal_error_callback(
    deps: DepsMut,
    response: String,
) -> NeutronResult<Response<NeutronMsg>> {
    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.push(response);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}
