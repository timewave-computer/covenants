use cosmwasm_std::{DepsMut, Binary, Response, from_json};
use neutron_sdk::{NeutronResult, bindings::msg::NeutronMsg};
use osmosis_std::types::osmosis::gamm::v1beta1::{QueryPoolResponse, Pool};
use polytone::callbacks::{ErrorResponse, ExecutionResponse};

use crate::{error::ContractError, state::{LATEST_OSMO_POOL_SNAPSHOT, CONTRACT_STATE, CALLBACKS}, msg::ContractState};


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
) -> NeutronResult<Response<NeutronMsg>> {
    let entries = match execute_callback_result {
        Ok(execution_response) => execution_response.result
            .into_iter()
            .map(|r| {
                match r.data {
                    Some(data) => data.to_string(),
                    None => "none".to_string(),
                }
            })
            .collect(),
        Err(err) => vec![err],
    };


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
