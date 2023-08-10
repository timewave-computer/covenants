use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::ibc::applications::transfer::v1::MsgTransfer;
use cosmos_sdk_proto::traits::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, CustomQuery, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128,
};
use covenant_clock::helpers::verify_clock;
use covenant_utils::neutron_ica::{SudoPayload, OpenAckVersion, RemoteChainInfo};
use cw2::set_contract_version;
use neutron_sdk::bindings::types::ProtobufAny;

use crate::msg::{
    ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{
    read_errors_from_queue, read_reply_payload,
    save_reply_payload, save_sudo_payload, ACKNOWLEDGEMENT_RESULTS, CLOCK_ADDRESS, CONTRACT_STATE,
    INTERCHAIN_ACCOUNTS, AUTOPILOT_FORMAT, NEXT_CONTRACT, REMOTE_CHAIN_INFO,
};
use neutron_sdk::{
    bindings::{
        msg::{MsgSubmitTxResponse, NeutronMsg},
        query::NeutronQuery,
    },
    interchain_txs::helpers::get_port_id,
    sudo::msg::{RequestPacket, SudoMsg},
    NeutronError, NeutronResult,
};

const INTERCHAIN_ACCOUNT_ID: &str = "stride-ica";

const CONTRACT_NAME: &str = "crates.io:covenant-ls";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // contract begins at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // validate and store other module addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    REMOTE_CHAIN_INFO.save(deps.storage, &RemoteChainInfo {
        connection_id: msg.neutron_stride_ibc_connection_id,
        channel_id: msg.stride_neutron_ibc_transfer_channel_id,
        denom: msg.ls_denom,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
        ica_timeout: msg.ica_timeout,
        ibc_fee: msg.ibc_fee,
    })?;

    Ok(Response::default()
        .add_attribute("method", "ls_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attribute("next_contract", next_contract)
        .add_attribute("ibc_transfer_timeout", msg.ibc_transfer_timeout)
        .add_attribute("ica_timeout", msg.ica_timeout))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {msg:?}").as_str());
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Transfer { amount } => {
            let ica_address = get_ica(deps.as_ref(), &env, INTERCHAIN_ACCOUNT_ID);
            match ica_address {
                Ok((_, _)) => {
                    try_execute_transfer(deps, env, info, amount)
                },
                Err(_) => {
                    Ok(Response::default()
                        .add_attribute("method", "try_permisionless_transfer")
                        .add_attribute("ica_status", "not_created")
                    )
                },
            }
        },
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_register_stride_ica(deps, env),
        ContractState::ICACreated => Ok(Response::default()),
    }
}

/// registers an interchain account on stride with port_id associated with `INTERCHAIN_ACCOUNT_ID`
fn try_register_stride_ica(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let register: NeutronMsg =
        NeutronMsg::register_interchain_account(remote_chain_info.connection_id, INTERCHAIN_ACCOUNT_ID.to_string());
    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);

    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_stride_ica")
        .add_message(register))
}

/// this is a permisionless transfer method. once liquid staked funds are in this
/// contract, anyone can call this method by passing an amount (`Uint128`) to transfer
/// the funds (with `ls_denom`) to the liquid pooler module.
fn try_execute_transfer(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    amount: Uint128,
) -> NeutronResult<Response<NeutronMsg>> {

    // first we verify whether the next contract is ready for receiving the funds
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query: Option<Addr> = deps.querier.query_wasm_smart(
        next_contract,
        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
    )?;

    // if query returns None, then we error and wait
    let deposit_address = if let Some(addr) = deposit_address_query {
        addr
    } else {
        return Err(NeutronError::Std(
            StdError::not_found("Next contract is not ready for receiving the funds yet")
        ))
    };

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

            let coin = Coin {
                denom: remote_chain_info.denom,
                amount: amount.to_string(),
            };

            // inner MsgTransfer that will be sent from stride to neutron.
            // because of this message delivery depending on the ica wrapper below,
            // timeout_timestamp = current block + ica timeout + ibc_transfer_timeout
            let msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: remote_chain_info.channel_id,
                token: Some(coin),
                sender: address,
                receiver: deposit_address.to_string(),
                timeout_height: None,
                timeout_timestamp: env
                    .block
                    .time
                    .plus_seconds(remote_chain_info.ica_timeout.u64())
                    .plus_seconds(remote_chain_info.ibc_transfer_timeout.u64())
                    .nanos(),
            };

            // Serialize the Transfer message
            let mut buf = Vec::new();
            buf.reserve(msg.encoded_len());
            if let Err(e) = msg.encode(&mut buf) {
                return Err(StdError::generic_err(format!("Encode error: {e}",)).into());
            }

            let protobuf = ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(buf),
            };

            // wrap the protobuf of MsgTransfer into a message to be executed
            // by our interchain account
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![protobuf],
                "".to_string(),
                remote_chain_info.ica_timeout.u64(),
                remote_chain_info.ibc_fee,
            );

            let sudo_msg = msg_with_sudo_callback(
                deps,
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "permisionless_transfer".to_string(),
                },
            )?;
            Ok(Response::default()
                .add_submessage(sudo_msg)
                .add_attribute("method", "try_execute_transfer")
            )
        }
        None => {
            // I can't think of a case of how we could end up here as `sudo_open_ack`
            // callback advances the state to `ICACreated` and stores the ICA.
            // just in case, we revert the state to `Instantiated` to restart the flow.
            CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
            Ok(Response::default()
                .add_attribute("method", "try_execute_transfer")
                .add_attribute("error", "no_ica_found")
            )
        },
    }
}

#[allow(unused)]
fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T>(
    deps: DepsMut,
    msg: C,
    payload: SudoPayload,
) -> StdResult<SubMsg<T>> {
    save_reply_payload(deps.storage, payload)?;
    Ok(SubMsg::reply_on_success(msg, SUDO_PAYLOAD_REPLY_ID))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ICAAddress {} => Ok(to_binary(&Addr::unchecked(
            get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID)?.0,
        ))?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => {
            let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
            
            // here we cover three cases:
            let ica = match INTERCHAIN_ACCOUNTS.may_load(deps.storage, key)? {
                Some(entry) => {
                    // 1. ICA had been created -> fetch the autopilot string and return Some(autopilot)
                    if let Some((addr, _)) = entry {
                        let autopilot_receiver = AUTOPILOT_FORMAT
                            .load(deps.storage)?
                            .replace("{st_ica}", &addr);
        
                        Some(autopilot_receiver)
                    }
                    // 2. ICA creation request had been submitted but did not receive
                    //    the channel_open_ack yet -> None
                    else {
                        None
                    }
                },
                // 3. ICA creation request hadn't been submitted yet -> None
                None => None,
            };
            // up to the querying module to make sense of the response
            Ok(to_binary(&ica)?)
        },
        QueryMsg::RemoteChainInfo {} => Ok(to_binary(&REMOTE_CHAIN_INFO.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo: received sudo msg: {msg:?}").as_str());

    match msg {
        // For handling successful (non-error) acknowledgements.
        SudoMsg::Response { request, data } => sudo_response(deps, request, data),

        // For handling error acknowledgements.
        SudoMsg::Error { request, details } => sudo_error(deps, request, details),

        // For handling error timeouts.
        SudoMsg::Timeout { request } => sudo_timeout(deps, env, request),

        // For handling successful registering of ICA
        SudoMsg::OpenAck {
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        } => sudo_open_ack(
            deps,
            env,
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        ),
        _ => Ok(Response::default()),
    }
}

// handler
fn sudo_open_ack(
    deps: DepsMut,
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
        return Err(StdError::generic_err("Can't parse counterparty_version"))
    };
    
    // Update the storage record associated with the interchain account.
    INTERCHAIN_ACCOUNTS.save(
        deps.storage,
        port_id,
        &Some((
            parsed_version.clone().address,
            parsed_version.clone().controller_connection_id,
        )),
    )?;
    CONTRACT_STATE.save(deps.storage, &ContractState::ICACreated)?;
    
    return Ok(Response::default()
       .add_attribute("method", "sudo_open_ack")
    )
}

fn sudo_response(deps: DepsMut, request: RequestPacket, data: Binary) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo_response: sudo received: {request:?} {data:?}",).as_str());

    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default().add_attribute("method", "sudo_response"))
}

fn sudo_timeout(deps: DepsMut, _env: Env, request: RequestPacket) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo timeout request: {request:?}").as_str());

    // revert the state to Instantiated to force re-creation of ICA
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // returning Ok as this is anticipated. channel is already closed.
    Ok(Response::default())
}

fn sudo_error(deps: DepsMut, request: RequestPacket, details: String) -> StdResult<Response> {
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

// prepare_sudo_payload is called from reply handler
// The method is used to extract sequence id and channel from SubmitTxResponse to
// process sudo payload defined in msg_with_sudo_callback later in Sudo handler.
// Such flow msg_with_sudo_callback() -> reply() -> prepare_sudo_payload() -> sudo()
// allows you "attach" some payload to your SubmitTx message
// and process this payload when an acknowledgement for the SubmitTx message
// is received in Sudo handler
fn prepare_sudo_payload(mut deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
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

fn get_ica(
    deps: Deps<impl CustomQuery>,
    env: &Env,
    interchain_account_id: &str,
) -> Result<(String, String), StdError> {
    let key = get_port_id(env.contract.address.as_str(), interchain_account_id);

    INTERCHAIN_ACCOUNTS
        .load(deps.storage, key)?
        .ok_or_else(|| StdError::generic_err("Interchain account is not created yet"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: reply msg: {msg:?}").as_str());
    match msg.id {
        SUDO_PAYLOAD_REPLY_ID => prepare_sudo_payload(deps, env, msg),
        _ => Err(StdError::generic_err(format!(
            "unsupported reply message id {}",
            msg.id
        ))),
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            stride_neutron_ibc_transfer_channel_id,
            next_contract,
            neutron_stride_ibc_connection_id,
            ls_denom,
            ibc_fee,
            ibc_transfer_timeout,
            ica_timeout,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(addr) = clock_addr {
                let addr = deps.api.addr_validate(&addr)?;
                CLOCK_ADDRESS.save(deps.storage, &addr)?;
                resp = resp.add_attribute("clock_addr", addr.to_string());
            }

            if let Some(channel_id) = stride_neutron_ibc_transfer_channel_id {
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.channel_id = channel_id.to_string();
                    Ok(info)
                })?;
                resp = resp.add_attribute("stride_neutron_ibc_transfer_channel_id", channel_id);
            }

            if let Some(addr) = next_contract {
                let addr = deps.api.addr_validate(&addr)?;
                resp = resp.add_attribute("next_contract", addr.to_string());
                NEXT_CONTRACT.save(deps.storage, &addr)?;
            }

            if let Some(connection_id) = neutron_stride_ibc_connection_id {
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.connection_id = connection_id.to_string();
                    Ok(info)
                })?;
                resp = resp.add_attribute("neutron_stride_ibc_connection_id", connection_id);
            }

            if let Some(denom) = ls_denom {
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.denom = denom.to_string();
                    Ok(info)
                })?;
                resp = resp.add_attribute("ls_denom", denom);
            }

            if let Some(timeout) = ibc_transfer_timeout {
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.ibc_transfer_timeout = timeout;
                    Ok(info)
                })?;
                resp = resp.add_attribute("ibc_transfer_timeout", timeout);
            }

            if let Some(timeout) = ica_timeout {
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.ica_timeout = timeout;
                    Ok(info)
                })?;
                resp = resp.add_attribute("ica_timeout", timeout);
            }

            if let Some(fee) = ibc_fee {
                if fee.ack_fee.is_empty() || fee.timeout_fee.is_empty() || !fee.recv_fee.is_empty()
                {
                    return Err(StdError::GenericErr {
                        msg: "invalid IbcFee".to_string(),
                    });
                }
                REMOTE_CHAIN_INFO.update(deps.storage, |mut info| -> StdResult<_>{
                    info.ibc_fee = fee.clone();
                    Ok(info)
                })?;
                resp = resp.add_attribute("ibc_fee_ack", fee.ack_fee[0].to_string());
                resp = resp.add_attribute("ibc_fee_timeout", fee.timeout_fee[0].to_string());
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