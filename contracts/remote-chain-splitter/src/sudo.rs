use cosmwasm_std::{Binary, DepsMut, Env, Reply, Response, StdError, StdResult};
use covenant_utils::neutron::OpenAckVersion;
use neutron_sdk::{
    bindings::{msg::MsgSubmitTxResponse, query::NeutronQuery},
    sudo::msg::RequestPacket,
};

use crate::{
    msg::ContractState,
    state::{read_reply_payload, save_sudo_payload, CONTRACT_STATE, INTERCHAIN_ACCOUNTS},
};

type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

// handler
pub fn sudo_open_ack(
    deps: ExecuteDeps,
    _env: Env,
    port_id: String,
    _channel_id: String,
    _counterparty_channel_id: String,
    counterparty_version: String,
) -> StdResult<Response> {
    // The version variable contains a JSON value with multiple fields,
    // including the generated account address.
    let parsed_version: Result<OpenAckVersion, _> =
        serde_json_wasm::from_str(counterparty_version.as_str());

    // get the parsed OpenAckVersion or return an error if we fail
    let Ok(parsed_version) = parsed_version else {
        return Err(StdError::generic_err("Can't parse counterparty_version"));
    };

    // Update the storage record associated with the interchain account.
    INTERCHAIN_ACCOUNTS.save(
        deps.storage,
        port_id,
        &Some((
            parsed_version.clone().address,
            parsed_version.controller_connection_id,
        )),
    )?;
    CONTRACT_STATE.save(deps.storage, &ContractState::IcaCreated)?;

    Ok(Response::default().add_attribute("method", "sudo_open_ack"))
}

pub fn sudo_response(
    deps: ExecuteDeps,
    request: RequestPacket,
    data: Binary,
) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo_response: sudo received: {request:?} {data:?}").as_str());

    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default().add_attribute("method", "sudo_response"))
}

pub fn sudo_timeout(deps: ExecuteDeps, _env: Env, request: RequestPacket) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo timeout request: {request:?}").as_str());

    // revert the state to Instantiated to force re-creation of ICA
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // returning Ok as this is anticipated. channel is already closed.
    Ok(Response::default())
}

pub fn sudo_error(
    deps: ExecuteDeps,
    request: RequestPacket,
    details: String,
) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo error: {details}").as_str());
    deps.api
        .debug(format!("WASMDEBUG: request packet: {request:?}").as_str());

    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default().add_attribute("method", "sudo_error"))
}

pub fn prepare_sudo_payload(mut deps: ExecuteDeps, _env: Env, msg: Reply) -> StdResult<Response> {
    let payload = read_reply_payload(deps.storage)?;
    let resp: MsgSubmitTxResponse = serde_json_wasm::from_slice(
        msg.result
            .into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("no result"))?
            .as_slice(),
    )
    .map_err(|e| StdError::generic_err(format!("failed to parse response: {e:?}")))?;
    deps.api
        .debug(format!("WASMDEBUG: reply msg: {resp:?}").as_str());
    let seq_id = resp.sequence_id;
    let channel_id = resp.channel;
    save_sudo_payload(deps.branch().storage, channel_id, seq_id, payload)?;
    Ok(Response::new())
}
