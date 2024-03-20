use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Binary, Coin, CosmosMsg, DepsMut, Env, QuerierWrapper, QueryRequest, Reply, Response, StdError,
    StdResult, Storage, SubMsg, Uint64,
};
use neutron_sdk::{
    bindings::{
        msg::{MsgSubmitTxResponse, NeutronMsg},
        query::NeutronQuery,
    },
    interchain_txs::helpers::get_port_id,
    sudo::msg::RequestPacket,
};

use crate::neutron::{OpenAckVersion, SudoPayload};

type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

pub trait IcaStateHelper {
    fn reset_state(&self, storage: &mut dyn Storage) -> StdResult<()>;
    fn clear_ica(&self, storage: &mut dyn Storage) -> StdResult<()>;
    fn save_ica(
        &self,
        storage: &mut dyn Storage,
        port_id: String,
        address: String,
        controller_connection_id: String,
    ) -> StdResult<()>;
    fn save_state_ica_created(&self, storage: &mut dyn Storage) -> StdResult<()>;
    fn save_reply_payload(&self, storage: &mut dyn Storage, payload: SudoPayload) -> StdResult<()>;
    fn read_reply_payload(&self, storage: &mut dyn Storage) -> StdResult<SudoPayload>;
    fn save_sudo_payload(
        &self,
        storage: &mut dyn Storage,
        channel_id: String,
        seq_id: u64,
        payload: SudoPayload,
    ) -> StdResult<()>;
    fn get_ica(&self, storage: &dyn Storage, key: String) -> StdResult<(String, String)>;
}

/// reverts th contract state to Instantiated and clears the ICA storage.
/// channel is already closed.
pub fn sudo_timeout<H: IcaStateHelper>(
    state_helper: &H,
    deps: ExecuteDeps,
    _env: Env,
    _request: RequestPacket,
) -> StdResult<Response<NeutronMsg>> {
    // revert the state to Instantiated to force re-creation of ICA
    state_helper.reset_state(deps.storage)?;
    state_helper.clear_ica(deps.storage)?;

    // returning Ok as this is anticipated. channel is already closed.
    Ok(Response::default())
}

/// handles the response. if request sequence or source channel are missing,
/// it will return an error and close the channel. otherwise returns an Ok()
/// with data encoded in base64 as a response attribute.
pub fn sudo_response(request: RequestPacket, data: Binary) -> StdResult<Response<NeutronMsg>> {
    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default()
        .add_attribute("method", "sudo_response")
        .add_attribute("data", data.to_base64()))
}

/// handles the sudo error. if request sequence or source channel are missing,
/// it will return an error and close the channel. otherwise returns an Ok().
pub fn sudo_error(request: RequestPacket, _details: String) -> StdResult<Response<NeutronMsg>> {
    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default().add_attribute("method", "sudo_error"))
}

pub fn sudo_open_ack<H: IcaStateHelper>(
    state_helper: &H,
    deps: ExecuteDeps,
    _env: Env,
    port_id: String,
    _channel_id: String,
    _counterparty_channel_id: String,
    counterparty_version: String,
) -> StdResult<Response<NeutronMsg>> {
    // The version variable contains a JSON value with multiple fields,
    // including the generated account address.
    let parsed_version: Result<OpenAckVersion, _> =
        serde_json_wasm::from_str(counterparty_version.as_str());

    // get the parsed OpenAckVersion or return an error if we fail
    let Ok(parsed_version) = parsed_version else {
        return Err(StdError::generic_err("Can't parse counterparty_version"));
    };

    state_helper.save_ica(
        deps.storage,
        port_id,
        parsed_version.address,
        parsed_version.controller_connection_id,
    )?;
    state_helper.save_state_ica_created(deps.storage)?;

    Ok(Response::default().add_attribute("method", "sudo_open_ack"))
}

/// prepare_sudo_payload is called from reply handler
/// The method is used to extract sequence id and channel from SubmitTxResponse to
/// process sudo payload defined in msg_with_sudo_callback later in Sudo handler.
/// Such flow msg_with_sudo_callback() -> reply() -> prepare_sudo_payload() -> sudo()
/// allows you "attach" some payload to your SubmitTx message
/// and process this payload when an acknowledgement for the SubmitTx message
/// is received in Sudo handler
pub fn prepare_sudo_payload<H: IcaStateHelper>(
    state_helper: &H,
    deps: ExecuteDeps,
    _env: Env,
    msg: Reply,
) -> StdResult<Response<NeutronMsg>> {
    let payload = state_helper.read_reply_payload(deps.storage)?;

    let resp: MsgSubmitTxResponse = serde_json_wasm::from_slice(
        msg.result
            .into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("no result"))?
            .as_slice(),
    )
    .map_err(|e| StdError::generic_err(format!("failed to parse response: {e:?}")))?;

    let seq_id = resp.sequence_id;
    let channel_id = resp.channel;

    state_helper.save_sudo_payload(deps.storage, channel_id, seq_id, payload)?;

    Ok(Response::default())
}

pub fn get_ica<H: IcaStateHelper>(
    state_helper: &H,
    storage: &dyn Storage,
    contract_addr: &str,
    ica_id: &str,
) -> StdResult<(String, String)> {
    let key = get_port_id(contract_addr, ica_id);
    state_helper.get_ica(storage, key)
}

pub fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T, H: IcaStateHelper>(
    state_helper: &H,
    deps: ExecuteDeps,
    msg: C,
    payload: SudoPayload,
    reply_id: u64,
) -> StdResult<SubMsg<T>> {
    state_helper.save_reply_payload(deps.storage, payload)?;
    Ok(SubMsg::reply_on_success(msg, reply_id))
}

// manual definitions for neutron ictxs module
#[cw_serde]
pub struct Params {
    pub msg_submit_tx_max_messages: Uint64,
    pub register_fee: Vec<Coin>,
}

#[cw_serde]
pub struct QueryParamsResponse {
    pub params: Params,
}

pub fn get_ictxs_module_params_query_msg() -> QueryRequest<NeutronQuery> {
    QueryRequest::Stargate {
        path: "/neutron.interchaintxs.v1.Query/Params".to_string(),
        data: Binary(Vec::new()),
    }
}

pub fn query_ica_registration_fee(
    querier: QuerierWrapper<'_, NeutronQuery>,
) -> StdResult<Vec<Coin>> {
    let query_msg = get_ictxs_module_params_query_msg();
    let response: QueryParamsResponse = querier.query(&query_msg)?;
    Ok(response.params.register_fee)
}
