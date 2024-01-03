use std::collections::HashSet;

use cosmos_sdk_proto::cosmos::bank::v1beta1::{Input, MsgMultiSend, Output};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, CosmosMsg, CustomQuery, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, Uint128,
};
use covenant_clock::helpers::verify_clock;
use covenant_utils::neutron_ica::{self, OpenAckVersion, RemoteChainInfo, SudoPayload};
use cw2::set_contract_version;
use neutron_sdk::bindings::msg::MsgSubmitTxResponse;
use neutron_sdk::interchain_txs::helpers::{
    decode_acknowledgement_response, decode_message_response, get_port_id,
};
use neutron_sdk::sudo::msg::{RequestPacket, SudoMsg};
use neutron_sdk::NeutronError;

use crate::msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    add_error_to_queue, read_reply_payload, read_sudo_payload, save_reply_payload,
    save_sudo_payload, CLOCK_ADDRESS, CONTRACT_STATE, INTERCHAIN_ACCOUNTS, REMOTE_CHAIN_INFO,
    SPLIT_CONFIG_MAP, TRANSFER_AMOUNT,
};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    NeutronResult,
};

const INTERCHAIN_ACCOUNT_ID: &str = "rc-ica";
const CONTRACT_NAME: &str = "crates.io:covenant-native-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    let remote_chain_info = RemoteChainInfo {
        connection_id: msg.remote_chain_connection_id,
        channel_id: msg.remote_chain_channel_id,
        denom: msg.denom,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
        ica_timeout: msg.ica_timeout,
        ibc_fee: msg.ibc_fee,
    };
    REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // validate each split and store it in a map
    let mut split_resp_attributes: Vec<Attribute> = Vec::with_capacity(msg.splits.len());
    let mut encountered_denoms: HashSet<String> = HashSet::with_capacity(msg.splits.len());

    for split in msg.splits {
        // if denom had not yet been encountered we proceed, otherwise error
        if encountered_denoms.insert(split.denom.to_string()) {
            let validated_split = split.validate()?;
            split_resp_attributes.push(validated_split.to_response_attribute());
            SPLIT_CONFIG_MAP.save(
                deps.storage,
                validated_split.denom,
                &validated_split.receivers,
            )?;
        } else {
            return Err(NeutronError::Std(StdError::GenericErr {
                msg: format!("multiple {:?} entries", split.denom),
            }));
        }
    }

    Ok(Response::default()
        .add_attribute("method", "native_splitter_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attributes(remote_chain_info.get_response_attributes())
        .add_attributes(split_resp_attributes))
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
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_register_ica(deps, env),
        ContractState::IcaCreated => try_split_funds(deps, env),
        ContractState::Completed => {
            Ok(Response::default().add_attribute("contract_state", "completed"))
        }
    }
}

fn try_register_ica(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let register: NeutronMsg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string(),
        None,
    );
    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);

    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_ica")
        .add_message(register))
}

fn try_split_funds(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
            let amount = TRANSFER_AMOUNT.load(deps.storage)?;
            let splits =
                SPLIT_CONFIG_MAP.load(deps.storage, remote_chain_info.denom.to_string())?;

            let mut outputs: Vec<Output> = Vec::with_capacity(splits.len());
            for split_receiver in splits.iter() {
                // get the fraction dedicated to this receiver
                let amt = amount
                    .checked_multiply_ratio(split_receiver.share, Uint128::new(100))
                    .map_err(|e| NeutronError::Std(StdError::GenericErr { msg: e.to_string() }))?;

                outputs.push(Output {
                    address: split_receiver.addr.to_string(),
                    coins: vec![Coin {
                        denom: remote_chain_info.denom.to_string(),
                        amount: amt.to_string(),
                    }],
                });
            }

            // todo: make sure output amounts add up to the input amount here
            let multi_send_msg = MsgMultiSend {
                inputs: vec![Input {
                    address,
                    coins: vec![Coin {
                        denom: remote_chain_info.denom,
                        amount: amount.to_string(),
                    }],
                }],
                outputs,
            };

            let protobuf = neutron_ica::to_proto_msg_multi_send(multi_send_msg)?;

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
                    message: "split_funds_msg".to_string(),
                },
            )?;
            Ok(Response::default()
                .add_submessage(sudo_msg)
                .add_attribute("method", "try_execute_split_funds"))
        }
        None => {
            // I can't think of a case of how we could end up here as `sudo_open_ack`
            // callback advances the state to `ICACreated` and stores the ICA.
            // just in case, we revert the state to `Instantiated` to restart the flow.
            CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
            Ok(Response::default()
                .add_attribute("method", "try_execute_split_funds")
                .add_attribute("error", "no_ica_found"))
        }
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
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => {
            let ica = query_deposit_address(deps, env)?;
            // up to the querying module to make sense of the response
            Ok(to_json_binary(&ica)?)
        }
        QueryMsg::RemoteChainInfo {} => {
            Ok(to_json_binary(&REMOTE_CHAIN_INFO.may_load(deps.storage)?)?)
        }
        QueryMsg::SplitConfig {} => {
            let vec = SPLIT_CONFIG_MAP
                .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .collect::<Result<Vec<_>, StdError>>()?;

            Ok(to_json_binary(&vec)?)
        }
        QueryMsg::TransferAmount {} => {
            Ok(to_json_binary(&TRANSFER_AMOUNT.may_load(deps.storage)?)?)
        }
        QueryMsg::IcaAddress {} => Ok(to_json_binary(
            &get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID)?.0,
        )?),
    }
}

fn query_deposit_address(deps: Deps<NeutronQuery>, env: Env) -> Result<Option<String>, StdError> {
    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    /*
       here we cover three possible cases:
       - 1. ICA had been created -> nice
       - 2. ICA creation request had been submitted but did not receive
           the channel_open_ack yet -> None
       - 3. ICA creation request hadn't been submitted yet -> None
    */
    match INTERCHAIN_ACCOUNTS.may_load(deps.storage, key)? {
        Some(Some((addr, _))) => Ok(Some(addr)), // case 1
        _ => Ok(None),                           // cases 2 and 3
    }
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

fn sudo_response(mut deps: DepsMut, request: RequestPacket, data: Binary) -> StdResult<Response> {
  let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;

    deps.api
        .debug(format!("WASMDEBUG: sudo_response: sudo received: {request:?} {data:?}",).as_str());

    let seq_id = request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    let channel_id = request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    let payload = read_sudo_payload(deps.storage, channel_id, seq_id).ok();
    if payload.is_none() {
        let error_msg = "WASMDEBUG: Error: Unable to read sudo payload";
        deps.api.debug(error_msg);
        add_error_to_queue(deps.storage, error_msg.to_string());
        return Ok(Response::default());
    }

    let parsed_data = decode_acknowledgement_response(data)?;

    // Iterate over the messages, parse them depending on their type & process them.
    let mut item_types = vec![];
    let mut complete_msg = vec![];

    for item in parsed_data {
        let item_type = item.msg_type.as_str();
        item_types.push(item_type.to_string());
        match item_type {
            "/cosmos.bank.v1beta1.MsgMultiSend" => {
                decode_message_response(&item.data)?;
                // TODO: look into if this successful decoding is enough to assume multi
                // send was successful
                complete_msg.push(ContractState::complete_and_dequeue(deps.branch(), clock_addr.as_str())?)
            }
            _ => {
                deps.api.debug(
                    format!("This type of acknowledgement is not implemented: {payload:?}")
                        .as_str(),
                );
            }
        }
    }

    Ok(Response::default().add_messages(complete_msg).add_attribute("method", "sudo_response"))
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
fn _prepare_sudo_payload(mut deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            remote_chain_info,
            splits,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(addr) = clock_addr {
                let clock_address = deps.api.addr_validate(&addr)?;
                CLOCK_ADDRESS.save(deps.storage, &clock_address)?;
                resp = resp.add_attribute("clock_addr", addr);
            }

            if let Some(remote_chain_info) = remote_chain_info {
                REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
                resp = resp.add_attribute("remote_chain_info", format!("{remote_chain_info:?}"));
            }

            if let Some(splits) = splits {
                let mut split_resp_attributes: Vec<Attribute> = Vec::with_capacity(splits.len());
                let mut encountered_denoms: HashSet<String> = HashSet::with_capacity(splits.len());

                for split in splits {
                    // if denom had not yet been encountered we proceed, otherwise error
                    if encountered_denoms.insert(split.denom.to_string()) {
                        let validated_split = split.validate()?;
                        split_resp_attributes.push(validated_split.to_response_attribute());
                        SPLIT_CONFIG_MAP.save(
                            deps.storage,
                            validated_split.denom.clone(),
                            &validated_split.receivers,
                        )?;

                        resp = resp.add_attribute(
                            format!("split-{}", validated_split.denom),
                            format!("{:?}", validated_split.receivers),
                        );
                    } else {
                        return Err(StdError::generic_err(format!(
                            "multiple {:?} entries",
                            split.denom
                        )));
                    }
                }
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => todo!(),
    }
}
