use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::ibc::applications::transfer::v1::MsgTransfer;

use cosmos_sdk_proto::ibc::core::client::v1::Height;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, CustomQuery, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg, Addr, Uint128,
};
use cw2::set_contract_version;
use neutron_sdk::bindings::types::ProtobufAny;
use neutron_sdk::interchain_queries::v045::new_register_transfers_query_msg;

use prost::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use neutron_sdk::bindings::msg::IbcFee;
use neutron_sdk::{
    bindings::{
        msg::{MsgSubmitTxResponse, NeutronMsg},
        query::{NeutronQuery, QueryInterchainAccountAddressResponse},
    },
    interchain_txs::helpers::{
        decode_acknowledgement_response, get_port_id,
    },
    sudo::msg::{RequestPacket, SudoMsg},
    NeutronError, NeutronResult,
};

use crate::state::{
    add_error_to_queue, read_errors_from_queue, read_reply_payload, read_sudo_payload,
    save_sudo_payload, AcknowledgementResult,
    ACKNOWLEDGEMENT_RESULTS, INTERCHAIN_ACCOUNTS, SUDO_PAYLOAD_REPLY_ID, CLOCK_ADDRESS, STRIDE_ATOM_RECEIVER, NATIVE_ATOM_RECEIVER, IBC_PORT_ID, ICA_ADDRESS, SudoPayload, save_reply_payload, CONTRACT_STATE, ContractState, GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID,
};

// Default timeout for SubmitTX is two weeks
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 60 * 24 * 7 * 2;
// const DEFAULT_TIMEOUT_HEIGHT: u64 = 10000000;
const NEUTRON_DENOM: &str = "untrn";
const ATOM_DENOM: &str = "uatom";
const IBC_CONNECTION: &str = "connection-0";
const ICS_CONNECTION_ID: &str = "connection-1";
const INTERCHAIN_ACCOUNT_ID: &str = "test";
const TRANSFER_PORT: &str = "transfer";

const CONTRACT_NAME: &str = "crates.io:covenant-depositor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct  OpenAckVersion {
    version: String,
    controller_connection_id: String,
    host_connection_id: String,
    address: String,
    encoding: String,
    tx_type: String,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: validations

    // minations and amounts
    STRIDE_ATOM_RECEIVER.save(deps.storage, &msg.st_atom_receiver)?;
    NATIVE_ATOM_RECEIVER.save(deps.storage, &msg.atom_receiver)?;
    CLOCK_ADDRESS.save(deps.storage, &Addr::unchecked(msg.clock_address))?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID.save(deps.storage, &msg.gaia_neutron_ibc_transfer_channel_id)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());
    match msg {
        ExecuteMsg::Tick {  } => try_tick(deps, env, info),
        ExecuteMsg::Received {  } =>  try_handle_received(),
    }
}


fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    let current_state = CONTRACT_STATE.load(deps.storage)?;
    let ica_address: Result<String, StdError> = ICA_ADDRESS.load(deps.storage);
    let gaia_account_address = match ica_address {
        Ok(addr) => addr,
        Err(_) => "todo".to_string(),
    };
    // TODO: validate caller is clock

    match current_state {
        ContractState::Instantiated => try_register_gaia_ica(deps, env),
        ContractState::ICACreated => try_receive_atom_from_ica(deps, env, info, gaia_account_address),
        ContractState::ReceivedFunds => try_execute_transfers(deps, env, info, gaia_account_address),
        ContractState::Complete => Ok(Response::default()),
    }
}

fn try_receive_atom_from_ica(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo, 
    _gaia_account_address: String
) -> NeutronResult<Response<NeutronMsg>> {
    let fee = IbcFee {
        recv_fee: vec![], // must be empty
        ack_fee: vec![cosmwasm_std::Coin { 
            denom: NEUTRON_DENOM.to_string(),
            amount: Uint128::new(1000u128)
        }],
        timeout_fee: vec![cosmwasm_std::Coin { 
            denom: NEUTRON_DENOM.to_string(), 
            amount: Uint128::new(1000u128) 
        }],
    };
    let port_id = IBC_PORT_ID.load(deps.storage)?;

    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;
    
    match interchain_account {
        Some((address, controller_conn_id)) => {
            let source_channel = GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID.load(deps.storage)?;
            let lp_receiver = NATIVE_ATOM_RECEIVER.load(deps.storage)?;
            let amount = String::from(lp_receiver.amount.to_string());
            // let receiver = String::from(lp_receiver.address.to_string());

            let coin = Coin {
                denom: ATOM_DENOM.to_string(),
                amount,
            };

            let msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: source_channel,
                token: Some(coin),
                sender: address.clone(),
                // receiver: String::from(lp_receiver.address.to_string()),
                receiver: env.contract.address.to_string(),
                // TODO: look into what the timeout_height should be
                timeout_height: Some(Height {
                    revision_number: 2, 
                    revision_height: 500,
                }),
                timeout_timestamp: 0,
            };
        
            // Serialize the Transfer message
            let mut buf = Vec::new();
            buf.reserve(msg.encoded_len());
            if let Err(e) = msg.encode(&mut buf) {
                return Err(StdError::generic_err(format!("Encode error: {}", e)).into());
            }
        
            let protobuf = ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(buf),
            };

            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id, 
                INTERCHAIN_ACCOUNT_ID.to_string(), 
                vec![protobuf], 
                address, 
                100000, 
                fee
            );

            Ok(Response::default()
                .add_submessage(SubMsg::new(submit_msg))
            )
        },
        None => return Err(NeutronError::Std(StdError::NotFound { kind: "no ica found".to_string() })),
    }   
}

fn try_register_gaia_ica(
    deps: DepsMut, 
    env: Env,
) -> NeutronResult<Response<NeutronMsg>> {
    let gaia_acc_id = INTERCHAIN_ACCOUNT_ID.to_string();
    // let ibc_connection_id = String::from("connection-0");
    let ics_connection_id = ICS_CONNECTION_ID.to_string();
    let register = NeutronMsg::register_interchain_account(
        ics_connection_id, 
        gaia_acc_id.clone()
    );
    let key = get_port_id(env.contract.address.as_str(), &gaia_acc_id);
    IBC_PORT_ID.save(deps.storage, &key)?;
    
    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new().add_message(register))
}

fn try_execute_transfers(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo, 
    gaia_account_address: String
) -> NeutronResult<Response<NeutronMsg>> {
    // validate that tick was triggered by the authorized clock
    // validate whether ICA has enough atom?

    let stride_atom_receiver = STRIDE_ATOM_RECEIVER.load(deps.branch().storage)?;
    let native_atom_receiver = NATIVE_ATOM_RECEIVER.load(deps.branch().storage)?;
    
    // match bal {
        // Ok(coin) => {
    // validate depositor ICA has enough atoms to perform both transfers?
    
    // 1. transfer 1/2 of atoms to liquid-staker module
    // 2. transfer 1/2 of atoms from ICA to liquidity-pooler module

    // let fee = min_ntrn_ibc_fee(query_min_ibc_fee(deps.as_ref())?.min_fee);
    let _neutron_coin = Coin {
        denom: NEUTRON_DENOM.to_string(),
        amount: 1000u128.to_string(),
    };
    let fee = IbcFee {
        recv_fee: vec![], // must be empty
        ack_fee: vec![cosmwasm_std::Coin { denom: NEUTRON_DENOM.to_string(), amount: Uint128::new(1000u128) }],
        timeout_fee: vec![cosmwasm_std::Coin { denom: NEUTRON_DENOM.to_string(), amount: Uint128::new(1000u128) }],
    };

    let ls_coin = Coin {
        denom: ATOM_DENOM.to_string(),
        amount: stride_atom_receiver.amount.to_string(),
    };
    let lp_coin = Coin {
        denom: ATOM_DENOM.to_string(),
        amount: native_atom_receiver.amount.to_string(),
    };

    let ls_msg = MsgTransfer {
        source_port: TRANSFER_PORT.to_string(),
        source_channel: "channel-0".to_string(),
        token: Some(ls_coin),
        sender: gaia_account_address.clone(),
        receiver: stride_atom_receiver.address,
        timeout_height: None,
        timeout_timestamp: 0,
    };
    
    let lp_msg = MsgTransfer {
        source_port: TRANSFER_PORT.to_string(),
        source_channel: "channel-0".to_string(),
        token: Some(lp_coin),
        sender: gaia_account_address.clone(),
        receiver: native_atom_receiver.address,
        timeout_height: None,
        timeout_timestamp: 0,
    };

    // Serialize the Transfer messages 
    let mut ls_buf = Vec::new();
    ls_buf.reserve(ls_msg.encoded_len());

    if let Err(e) = ls_msg.encode(&mut ls_buf) {
        return Err(StdError::generic_err(format!("Encode error: {}", e)).into());
    }

    let mut lp_buf = Vec::new();
    lp_buf.reserve(lp_msg.encoded_len());

    if let Err(e) = lp_msg.encode(&mut lp_buf) {
        return Err(StdError::generic_err(format!("Encode error: {}", e)).into());
    }

    let ls_protobuf = ProtobufAny {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary::from(ls_buf),
    };  
    let lp_protobuf = ProtobufAny {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary::from(lp_buf),
    };

    let ls_cosmos_msg = NeutronMsg::submit_tx(
        IBC_CONNECTION.to_string(),
        INTERCHAIN_ACCOUNT_ID.to_string(),
        vec![ls_protobuf],
        "".to_string(),
        DEFAULT_TIMEOUT_SECONDS,
        fee.clone()
    );
    let lp_cosmos_msg = NeutronMsg::submit_tx(
        IBC_CONNECTION.to_string(),
        INTERCHAIN_ACCOUNT_ID.to_string(),
        vec![lp_protobuf],
        "".to_string(),
        DEFAULT_TIMEOUT_SECONDS,
        fee
    );

    let ls_submsg = msg_with_sudo_callback(
        deps.branch(), 
        ls_cosmos_msg,
        SudoPayload {
            port_id: get_port_id(
                env.contract.address.to_string(), 
                INTERCHAIN_ACCOUNT_ID.to_string(),
            ),
            // Here you can store some information about the transaction to help you parse
            // the acknowledgement later.
            message: "ls transfer".to_string(),  
        },
    )?;

    let lp_submsg = msg_with_sudo_callback(
        deps,
        lp_cosmos_msg,
        SudoPayload {
            port_id: get_port_id(
                env.contract.address.to_string(), 
                INTERCHAIN_ACCOUNT_ID.to_string()
            ),
            // Here you can store some information about the transaction to help you parse
            // the acknowledgement later.
            message: "lp transfer".to_string(),  
        },
    )?;

    Ok(Response::default()
        .add_submessages(vec![ls_submsg, lp_submsg])
    )
}

fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T>(
    deps: DepsMut,
    msg: C,
    payload: SudoPayload,
) -> StdResult<SubMsg<T>> {
    save_reply_payload(deps.storage, payload)?;
    Ok(SubMsg::reply_on_success(msg, SUDO_PAYLOAD_REPLY_ID))
}

pub fn register_transfers_query(
    connection_id: String,
    recipient: String,
    update_period: u64,
    min_height: Option<u64>,
) -> NeutronResult<Response<NeutronMsg>> {
    let msg =
        new_register_transfers_query_msg(connection_id, recipient, update_period, min_height)?;

    Ok(Response::new().add_message(msg))
}

fn try_handle_received() -> NeutronResult<Response<NeutronMsg>> {

    Ok(Response::default().add_attribute("try_handle_received", "received msg`"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::StAtomReceiver {} => Ok(
            to_binary(&STRIDE_ATOM_RECEIVER.may_load(deps.storage)?)?
        ),
        QueryMsg::AtomReceiver {} => Ok(
            to_binary(&NATIVE_ATOM_RECEIVER.may_load(deps.storage)?)?
        ),
        QueryMsg::ClockAddress {} => Ok(
            to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?
        ),
        QueryMsg::DepositorInterchainAccountAddress {} => query_depositor_interchain_address(deps, env), 
        QueryMsg::InterchainAccountAddress {
            interchain_account_id,
            connection_id,
        } => query_interchain_address(deps, env, interchain_account_id, connection_id),
        QueryMsg::InterchainAccountAddressFromContract {
            interchain_account_id,
        } => query_interchain_address_contract(deps, env, interchain_account_id),
        QueryMsg::AcknowledgementResult {
            interchain_account_id,
            sequence_id,
        } => query_acknowledgement_result(deps, env, interchain_account_id, sequence_id),
        QueryMsg::ErrorsQueue {} => query_errors_queue(deps),
    }
}

pub fn query_depositor_interchain_address(
    deps: Deps<NeutronQuery>,
    _env: Env,
) -> NeutronResult<Binary> {
    let addr = ICA_ADDRESS.load(deps.storage);

    match addr {
        Ok(val) => {
            let address_response = QueryInterchainAccountAddressResponse {
                interchain_account_address: val,
            };
            Ok(to_binary(&address_response)?)
        },
        Err(_) => Err(NeutronError::Std(StdError::not_found("no ica stored"))),
    }
    
    // let gaia_acc_id = String::from("gaia-acc");
    // let ics_connection_id = String::from("connection-1");
    // let account_key = get_port_id(env.contract.address.as_str(), &gaia_acc_id);
    // let interchain_account_addr = INTERCHAIN_ACCOUNTS.load(deps.storage, account_key)?;
    
    // query_interchain_address(deps, env, gaia_acc_id, ics_connection_id)
    // let query = NeutronQuery::InterchainAccountAddress {
    //     owner_address: env.contract.address.to_string(),
    //     interchain_account_id: gaia_acc_id,
    //     connection_id: ics_connection_id,
    // };
    
    // let res: QueryInterchainAccountAddressResponse = deps.querier.query(&query.into())?;
}

// returns ICA address from Neutron ICA SDK module
pub fn query_interchain_address(
    deps: Deps<NeutronQuery>,
    env: Env,
    interchain_account_id: String,
    connection_id: String,
) -> NeutronResult<Binary> {
    let query = NeutronQuery::InterchainAccountAddress {
        owner_address: env.contract.address.to_string(),
        interchain_account_id,
        connection_id,
    };

    let res: QueryInterchainAccountAddressResponse = deps.querier.query(&query.into())?;
    Ok(to_binary(&res)?)
}

// returns ICA address from the contract storage. The address was saved in sudo_open_ack method
pub fn query_interchain_address_contract(
    deps: Deps<NeutronQuery>,
    env: Env,
    interchain_account_id: String,
) -> NeutronResult<Binary> {
    Ok(to_binary(&get_ica(deps, &env, &interchain_account_id)?)?)
}

// returns the result
pub fn query_acknowledgement_result(
    deps: Deps<NeutronQuery>,
    env: Env,
    interchain_account_id: String,
    sequence_id: u64,
) -> NeutronResult<Binary> {
    let port_id = get_port_id(env.contract.address.as_str(), &interchain_account_id);
    let res = ACKNOWLEDGEMENT_RESULTS.may_load(deps.storage, (port_id, sequence_id))?;
    Ok(to_binary(&res)?)
}

pub fn query_errors_queue(deps: Deps<NeutronQuery>) -> NeutronResult<Binary> {
    let res = read_errors_from_queue(deps.storage)?;
    Ok(to_binary(&res)?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo: received sudo msg: {:?}", msg).as_str());

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    Ok(Response::default())
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

    // Update the storage record associated with the interchain account.
    if let Ok(parsed_version) = parsed_version {
        INTERCHAIN_ACCOUNTS.save(
            deps.storage,
            port_id,
            &Some((
                parsed_version.clone().address,
                parsed_version.clone().controller_connection_id,
            )),
        )?;
        ICA_ADDRESS.save(deps.storage, &parsed_version.address)?;
        CONTRACT_STATE.save(deps.storage, &ContractState::ICACreated)?;
        return Ok(Response::default());
    }
    Err(StdError::generic_err("Can't parse counterparty_version"))
}

fn sudo_response(deps: DepsMut, request: RequestPacket, data: Binary) -> StdResult<Response> {
    deps.api.debug(
        format!(
            "WASMDEBUG: sudo_response: sudo received: {:?} {:?}",
            request, data
        )
        .as_str(),
    );

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let seq_id = request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let channel_id = request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    // NOTE: NO ERROR IS RETURNED HERE. THE CHANNEL LIVES ON.
    // In this particular example, this is a matter of developer's choice. Not being able to read
    // the payload here means that there was a problem with the contract while submitting an
    // interchain transaction. You can decide that this is not worth killing the channel,
    // write an error log and / or save the acknowledgement to an errors queue for later manual
    // processing. The decision is based purely on your application logic.
    let payload = read_sudo_payload(deps.storage, channel_id, seq_id).ok();
    if payload.is_none() {
        let error_msg = "WASMDEBUG: Error: Unable to read sudo payload";
        deps.api.debug(error_msg);
        add_error_to_queue(deps.storage, error_msg.to_string());
        return Ok(Response::default());
    }

    deps.api
        .debug(format!("WASMDEBUG: sudo_response: sudo payload: {:?}", payload).as_str());

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not being able to parse this data
    // that a fatal error occurred on Neutron side, or that the remote chain sent us unexpected data.
    // Both cases require immediate attention.
    let parsed_data = decode_acknowledgement_response(data)?;

    let mut item_types = vec![];
    for item in parsed_data {
        let item_type = item.msg_type.as_str();
        item_types.push(item_type.to_string());
        match item_type {
            "/ibc.applications.transfer.v1.MsgTransfer" => {
                deps.api.debug(
                    format!("MsgTransfer response: {:?}", item.data).as_str()
                );
            },
            _ => {
                deps.api.debug(
                    format!(
                        "This type of acknowledgement is not implemented: {:?}",
                        payload
                    )
                    .as_str(),
                );
            }
        }
    }

    if let Some(payload) = payload {
        // update but also check that we don't update same seq_id twice
        ACKNOWLEDGEMENT_RESULTS.update(
            deps.storage,
            (payload.port_id, seq_id),
            |maybe_ack| -> StdResult<AcknowledgementResult> {
                match maybe_ack {
                    Some(_ack) => Err(StdError::generic_err("trying to update same seq_id")),
                    None => Ok(AcknowledgementResult::Success(item_types)),
                }
            },
        )?;
    }

    Ok(Response::default())
}

fn sudo_timeout(deps: DepsMut, _env: Env, request: RequestPacket) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo timeout request: {:?}", request).as_str());

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let seq_id = request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let channel_id = request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    // update but also check that we don't update same seq_id twice
    // NOTE: NO ERROR IS RETURNED HERE. THE CHANNEL LIVES ON.
    // In this particular example, this is a matter of developer's choice. Not being able to read
    // the payload here means that there was a problem with the contract while submitting an
    // interchain transaction. You can decide that this is not worth killing the channel,
    // write an error log and / or save the acknowledgement to an errors queue for later manual
    // processing. The decision is based purely on your application logic.
    // Please be careful because it may lead to an unexpected state changes because state might
    // has been changed before this call and will not be reverted because of supressed error.
    let payload = read_sudo_payload(deps.storage, channel_id, seq_id).ok();
    if let Some(payload) = payload {
        // update but also check that we don't update same seq_id twice
        ACKNOWLEDGEMENT_RESULTS.update(
            deps.storage,
            (payload.port_id, seq_id),
            |maybe_ack| -> StdResult<AcknowledgementResult> {
                match maybe_ack {
                    Some(_ack) => Err(StdError::generic_err("trying to update same seq_id")),
                    None => Ok(AcknowledgementResult::Timeout(payload.message)),
                }
            },
        )?;
    } else {
        let error_msg = "WASMDEBUG: Error: Unable to read sudo payload";
        deps.api.debug(error_msg);
        add_error_to_queue(deps.storage, error_msg.to_string());
    }

    Ok(Response::default())
}

fn sudo_error(deps: DepsMut, request: RequestPacket, details: String) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo error: {}", details).as_str());
    deps.api
        .debug(format!("WASMDEBUG: request packet: {:?}", request).as_str());

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let seq_id = request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    // WARNING: RETURNING THIS ERROR CLOSES THE CHANNEL.
    // AN ALTERNATIVE IS TO MAINTAIN AN ERRORS QUEUE AND PUT THE FAILED REQUEST THERE
    // FOR LATER INSPECTION.
    // In this particular case, we return an error because not having the sequence id
    // in the request value implies that a fatal error occurred on Neutron side.
    let channel_id = request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;
    let payload = read_sudo_payload(deps.storage, channel_id, seq_id).ok();

    if let Some(payload) = payload {
        // update but also check that we don't update same seq_id twice
        ACKNOWLEDGEMENT_RESULTS.update(
            deps.storage,
            (payload.port_id, seq_id),
            |maybe_ack| -> StdResult<AcknowledgementResult> {
                match maybe_ack {
                    Some(_ack) => Err(StdError::generic_err("trying to update same seq_id")),
                    None => Ok(AcknowledgementResult::Error((payload.message, details))),
                }
            },
        )?;
    } else {
        let error_msg = "WASMDEBUG: Error: Unable to read sudo payload";
        deps.api.debug(error_msg);
        add_error_to_queue(deps.storage, error_msg.to_string());
    }

    Ok(Response::default())
}

// prepare_sudo_payload is called from reply handler
// The method is used to extract sequence id and channel from SubmitTxResponse to process sudo payload defined in msg_with_sudo_callback later in Sudo handler.
// Such flow msg_with_sudo_callback() -> reply() -> prepare_sudo_payload() -> sudo() allows you "attach" some payload to your SubmitTx message
// and process this payload when an acknowledgement for the SubmitTx message is received in Sudo handler
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
    .map_err(|e| StdError::generic_err(format!("failed to parse response: {:?}", e)))?;
    deps.api
        .debug(format!("WASMDEBUG: reply msg: {:?}", resp).as_str());
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

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: reply msg: {:?}", msg).as_str());
    match msg.id {
        SUDO_PAYLOAD_REPLY_ID => prepare_sudo_payload(deps, env, msg),
        _ => Err(StdError::generic_err(format!(
            "unsupported reply message id {}",
            msg.id
        ))),
    }
}
    