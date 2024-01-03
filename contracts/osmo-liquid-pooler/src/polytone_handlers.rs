use cosmwasm_std::{DepsMut, Binary, Response, from_json};
use neutron_sdk::{NeutronResult, bindings::msg::NeutronMsg};
use osmosis_std::types::osmosis::gamm::v1beta1::{QueryPoolResponse, Pool};
use polytone::callbacks::{ErrorResponse, ExecutionResponse};

use crate::{state::{LATEST_OSMO_POOL_SNAPSHOT, CONTRACT_STATE, CALLBACKS}, msg::ContractState};


pub fn process_query_callback(
    deps: DepsMut,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
) -> NeutronResult<Response<NeutronMsg>> {
    let entries = match query_callback_result {
        Ok(response) => {
            if let Some(query_response_b64) = response.last() {
                let res: QueryPoolResponse = from_json(query_response_b64)?;
                // todo: remove unwraps
                let pool: Pool = res.pool
                    .unwrap()
                    .try_into()
                    .unwrap();
                LATEST_OSMO_POOL_SNAPSHOT.save(deps.storage, &Some(pool))?;
                CONTRACT_STATE.save(deps.storage, &ContractState::ProxyFunded)?;
            };

            response.into_iter().map(|resp| resp.to_string()).collect()
        },
        Err(err) => vec![format!("{:?} : {:?}", err.message_index, err.error)],
    };

    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
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
