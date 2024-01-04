use std::str::FromStr;

use cosmwasm_std::{DepsMut, Binary, Response, from_json, Uint128, Coin, Uint64};
use neutron_sdk::{NeutronResult, bindings::msg::NeutronMsg};
use osmosis_std::types::{osmosis::gamm::v1beta1::{QueryPoolResponse, Pool}, cosmos::{bank::v1beta1::QueryBalanceResponse}};
use polytone::callbacks::{ErrorResponse, ExecutionResponse};

use crate::{state::{LATEST_OSMO_POOL_SNAPSHOT, CONTRACT_STATE, CALLBACKS, LATEST_PROXY_BALANCES}, msg::ContractState};


pub fn process_query_callback(
    deps: DepsMut,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    // either query_pool or proxy_balances
    let initiator_msg: String = from_json(initiator_msg)?;
    let mut coin_balances = vec![];

    let entries = match query_callback_result {
        Ok(response) => {
            let mut responses = vec![];
            for resp in response {
                if initiator_msg == "query_pool" {
                    let res: QueryPoolResponse = from_json(resp.clone())?;
                    // todo: remove unwraps
                    let pool: Pool = res.pool
                        .unwrap()
                        .try_into()
                        .unwrap();
                    LATEST_OSMO_POOL_SNAPSHOT.save(deps.storage, &Some(pool))?;
                } else if initiator_msg == "proxy_balances" {
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
        },
        Err(err) => vec![format!("{:?} : {:?}", err.message_index, err.error)],
    };

    if coin_balances.len() > 0 {
        LATEST_PROXY_BALANCES.save(deps.storage, &Some(coin_balances))?;
    }

    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
    callbacks.push(initiator_msg);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

pub fn process_execute_callback(
    deps: DepsMut,
    execute_callback_result: Result<ExecutionResponse, String>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    let mut entries: Vec<String> = vec![];
    let initiator_msg: String = from_json(initiator_msg)?;
    match execute_callback_result {
        Ok(execution_response) => {
            for result in execution_response.result {
                let decoded = match result.data {
                    Some(data) => {
                        if initiator_msg == "liquidity_provided" {
                            entries.push("liquidity_provided".to_string());
                        } else if initiator_msg == "proxy_created" {
                            entries.push("proxy_created".to_string());
                        }
                        data.to_string()
                    },
                    None => "none".to_string(),
                };
                entries.push(decoded);
            };
        },
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
