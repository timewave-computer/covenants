use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::ibc::applications::transfer::v1::MsgTransfer;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg,
};
use covenant_clock::helpers::verify_clock;
use cw2::set_contract_version;
use neutron_sdk::bindings::types::ProtobufAny;
use neutron_sdk::interchain_queries::v045::new_register_transfers_query_msg;

use prost::Message;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, OpenAckVersion, QueryMsg, ContractState, SudoPayload, AcknowledgementResult},
    state::{
        IBC_TRANSFER_TIMEOUT, ICA_TIMEOUT, NEUTRON_ATOM_IBC_DENOM, PENDING_NATIVE_TRANSFER_TIMEOUT,
    },
};
use neutron_sdk::{
    bindings::{
        msg::{MsgSubmitTxResponse, NeutronMsg},
        query::{NeutronQuery, QueryInterchainAccountAddressResponse},
    },
    interchain_txs::helpers::{decode_acknowledgement_response, get_port_id},
    sudo::msg::{RequestPacket, SudoMsg},
    NeutronError, NeutronResult,
};

use crate::state::{
    add_error_to_queue, read_errors_from_queue, read_reply_payload, read_sudo_payload,
    save_reply_payload, save_sudo_payload,
    ACKNOWLEDGEMENT_RESULTS, AUTOPILOT_FORMAT, CLOCK_ADDRESS, CONTRACT_STATE,
    GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID, GAIA_STRIDE_IBC_TRANSFER_CHANNEL_ID, IBC_FEE,
    INTERCHAIN_ACCOUNTS, LS_ADDRESS, NATIVE_ATOM_RECEIVER,
    NEUTRON_GAIA_CONNECTION_ID, STRIDE_ATOM_RECEIVER,
};

type QueryDeps<'a> = Deps<'a, NeutronQuery>;
type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

const ATOM_DENOM: &str = "uatom";
pub(crate) const INTERCHAIN_ACCOUNT_ID: &str = "ica";

pub const SUDO_PAYLOAD_REPLY_ID: u64 = 1;

const CONTRACT_NAME: &str = "crates.io:covenant-depositor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
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
    let ls_addr = deps.api.addr_validate(&msg.ls_address)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LS_ADDRESS.save(deps.storage, &ls_addr)?;

    // store information needed to forward funds to next modules
    STRIDE_ATOM_RECEIVER.save(deps.storage, &msg.st_atom_receiver)?;
    NATIVE_ATOM_RECEIVER.save(deps.storage, &msg.atom_receiver)?;
    NEUTRON_ATOM_IBC_DENOM.save(deps.storage, &msg.neutron_atom_ibc_denom)?;

    // store the channel and connection ids for ibc transactions
    GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID
        .save(deps.storage, &msg.gaia_neutron_ibc_transfer_channel_id)?;
    GAIA_STRIDE_IBC_TRANSFER_CHANNEL_ID
        .save(deps.storage, &msg.gaia_stride_ibc_transfer_channel_id)?;
    NEUTRON_GAIA_CONNECTION_ID.save(deps.storage, &msg.neutron_gaia_connection_id)?;
    
    // autopilot string formatting
    AUTOPILOT_FORMAT.save(deps.storage, &msg.autopilot_format)?;
    
    // ibc fees and timeouts
    IBC_FEE.save(deps.storage, &msg.ibc_fee)?;
    ICA_TIMEOUT.save(deps.storage, &msg.ica_timeout)?;
    IBC_TRANSFER_TIMEOUT.save(deps.storage, &msg.ibc_transfer_timeout)?;

    Ok(Response::default()
        .add_attribute("method", "depositor_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attribute("ls_address", ls_addr)
        .add_attribute("neutron_atom_ibc_denom", msg.neutron_atom_ibc_denom)
        .add_attribute("gaia_neutron_ibc_transfer_channel_id", msg.gaia_neutron_ibc_transfer_channel_id)
        .add_attribute("gaia_stride_ibc_transfer_channel_id", msg.gaia_stride_ibc_transfer_channel_id)
        .add_attribute("neutron_gaia_connection_id", msg.neutron_gaia_connection_id)
        .add_attribute("autopilot_format", msg.autopilot_format)
        .add_attribute("ica_timeout", msg.ica_timeout)
        .add_attribute("ibc_transfer_timeout", msg.ibc_transfer_timeout)

    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
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

/// attempts to advance the state machine. validates the caller to be the clock.
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;

    match current_state {
        ContractState::Instantiated => try_register_gaia_ica(deps, env),
        ContractState::ICACreated => {
            let ica_address = get_ica(deps.as_ref(), &env, INTERCHAIN_ACCOUNT_ID);
            match ica_address {
                Ok((_, _)) => {
                    try_send_native_token(env, deps)
                },
                Err(_) => {
                    Ok(Response::default()
                        .add_attribute("method", "try_tick")
                        .add_attribute("ica_status", "not_created")
                    )
                },
            }
        }
        ContractState::VerifyNativeToken => try_verify_native_token(env, deps),
        ContractState::VerifyLp => try_verify_lp(env, deps),
        ContractState::Complete => {
            Ok(Response::default().add_attribute("status", "function_completed"))
        },
    }
}

/// helper that serializes a MsgTransfer to protobuf
fn to_proto_msg_transfer(msg: impl Message) -> NeutronResult<ProtobufAny> {
    // Serialize the Transfer message
    let mut buf = Vec::new();
    buf.reserve(msg.encoded_len());
    if let Err(e) = msg.encode(&mut buf) {
        return Err(StdError::generic_err(format!("Encode error: {e}")).into());
    }

    Ok(ProtobufAny {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary::from(buf),
    })
}

/// attempts to forward the funds to LP module
fn try_send_native_token(env: Env, mut deps: ExecuteDeps) -> NeutronResult<Response<NeutronMsg>> {
    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let ibc_transfer_timeout = IBC_TRANSFER_TIMEOUT.load(deps.storage)?;
            let ica_timeout = ICA_TIMEOUT.load(deps.storage)?;
            let source_channel = GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID.load(deps.storage)?;
            let receiver = NATIVE_ATOM_RECEIVER.load(deps.storage)?;
            let fee = IBC_FEE.load(deps.storage)?;

            let coin = Coin {
                denom: ATOM_DENOM.to_string(),
                amount: receiver.amount.to_string(),
            };

            // we define the gaia->neutron timeout to be equal to:
            // current block + ICA timeout + ibc transfer timeout.
            // this assumes the worst possible time of delivery for the ICA message
            // which wraps the underlying MsgTransfer.
            let msg_transfer_timeout = env
                .block
                .time
                // we take the wrapping ICA tx timeout into account and assume the worst
                .plus_seconds(ica_timeout.u64())
                // and then add the preset ibc transfer timeout
                .plus_seconds(ibc_transfer_timeout.u64());

            // we store that timeout for later validation of pending transfers
            PENDING_NATIVE_TRANSFER_TIMEOUT.save(deps.storage, &msg_transfer_timeout)?;

            // transfer message that will send funds from the ICA on gaia to our LP module
            let lper_msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel,
                token: Some(coin),
                sender: address,
                receiver: receiver.address,
                timeout_height: None,
                timeout_timestamp: msg_transfer_timeout.nanos(),
            };

            let lp_protobuf = to_proto_msg_transfer(lper_msg)?;

            // tx to our ICA that wraps the transfer message defined above
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![lp_protobuf],
                "".to_string(),
                ica_timeout.u64(),
                fee,
            );

            let submsg = msg_with_sudo_callback(
                deps.branch(),
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "try_send_native_token".to_string(),
                },
            )?;

            Ok(Response::default()
                .add_attribute("method", "try_send_native_token")
                .add_submessage(submsg))
        }
        None => Ok(Response::default()
            .add_attribute("method", "try_send_native_token")
            .add_attribute("error", "no_ica_found")),
    }
}

fn query_lper_balance(deps: QueryDeps, lper: &str) -> StdResult<cosmwasm_std::Coin> {
    let neutron_atom_ibc_denom = NEUTRON_ATOM_IBC_DENOM.load(deps.storage)?;
    deps.querier.query_balance(lper, neutron_atom_ibc_denom)
}

/// we attempt to send funds to the ICA on stride
fn try_send_ls_token(env: Env, mut deps: ExecuteDeps) -> NeutronResult<SubMsg<NeutronMsg>> {
    // first we load the LS module address which is responsible for creating
    // an ICA on stride so that we can query for that ICA address
    let ls_address = LS_ADDRESS.load(deps.storage)?;
    let stride_ica_query: Option<String> = deps
        .querier
        .query_wasm_smart(ls_address, &covenant_ls::msg::QueryMsg::StrideICA {})?;
    let stride_ica_addr = match stride_ica_query {
        Some(addr) => addr,
        None => return Err(NeutronError::Std(StdError::not_found("no LS ica found"))),
    };

    // update the stride receiver to reflect where the funds are about to be sent to
    let stride_receiver = STRIDE_ATOM_RECEIVER.update(deps.storage, |mut r| -> StdResult<_> {
        r.address = stride_ica_addr.to_string();
        Ok(r)
    })?;

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let gaia_stride_channel = GAIA_STRIDE_IBC_TRANSFER_CHANNEL_ID.load(deps.storage)?;
            let ibc_transfer_timeout = IBC_TRANSFER_TIMEOUT.load(deps.storage)?;
            let ica_timeout = ICA_TIMEOUT.load(deps.storage)?;
            let fee = IBC_FEE.load(deps.storage)?;

            let stride_coin = Coin {
                denom: ATOM_DENOM.to_string(),
                amount: stride_receiver.amount.to_string(),
            };

            // we load the stored format string of autopilot and replace the dynamic fields
            // with our queried data
            let autopilot_receiver = AUTOPILOT_FORMAT
                .load(deps.storage)?
                .replace("{st_ica}", &stride_ica_addr);

            // transfer message that will send funds from the ICA on gaia to our ICA on stride
            let stride_msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: gaia_stride_channel,
                token: Some(stride_coin),
                sender: address,
                receiver: autopilot_receiver,
                timeout_height: None,
                timeout_timestamp: env
                    .block
                    .time
                    .plus_seconds(ibc_transfer_timeout.u64())
                    .nanos(),
            };

            let stride_protobuf = to_proto_msg_transfer(stride_msg)?;

            // tx to our ICA that wraps the transfer message defined above
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![stride_protobuf],
                "".to_string(),
                ica_timeout.u64(),
                fee,
            );

            Ok(msg_with_sudo_callback(
                deps.branch(),
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "try_send_st_token".to_string(),
                },
            )?)
        }
        None => Err(NeutronError::Std(StdError::not_found("no ica found"))),
    }
}

/// attempts to advance the state machine past the sending native tokens to LP module phase.
/// it queries the balances of the LP module and validates the amount there against our
/// expectations. if funds are not yet there, the timeout of previous transfer is validated,
/// taking an extra 5 minutes buffer into account.
/// if timeout is not yet due, and the funds did not arrive, we wait.
fn try_verify_native_token(env: Env, deps: ExecuteDeps) -> NeutronResult<Response<NeutronMsg>> {
    let receiver = NATIVE_ATOM_RECEIVER.load(deps.storage)?;
    let lper_native_token_balance = query_lper_balance(deps.as_ref(), &receiver.address)?;
    let pending_transfer_timeout = PENDING_NATIVE_TRANSFER_TIMEOUT.load(deps.storage)?;

    if lper_native_token_balance.amount >= receiver.amount {
        // if funds have arrived on LP module, we advance the state
        CONTRACT_STATE.save(deps.storage, &ContractState::VerifyLp)?;

        return Ok(Response::default()
            .add_attribute("method", "try_verify_native_token")
            .add_attribute("receiver_balance", lper_native_token_balance.amount));
    } else if env.block.time.nanos() >= pending_transfer_timeout.plus_minutes(5).nanos() {
        // funds are still not on the LP module and the msgTransfer timeout is due
        // we can safely retry sending the funds again by reverting the state
        // to ICACreated
        CONTRACT_STATE.save(deps.storage, &ContractState::ICACreated)?;
        return Ok(Response::default()
            .add_attribute("method", "try_verify_native_token")
            .add_attribute("status", "pending_transfer_timeout_due")
            .add_attribute("contract_state", "ica_created")
        );
    }

    // if tokens native tokens did not yet arrive to the LP module and the
    // timeout is not yet expired, we wait
    Ok(Response::default()
        .add_attribute("method", "try_verify_native_token")
        .add_attribute("status", "native_token_not_received"))
}

/// attempts to advance the state machine to its completed phase.
/// it does so by querying the LP module for its balance of the native tokens.
/// the expectation is for all (LS and native) tokens to be liquid staked, resulting
/// in zero balances of the tokens. we therefore expect the native token balance to be
/// zero and complete. this works because in previous states we queried and asserted
/// the native token balance to be non-zero.
fn try_verify_lp(env: Env, deps: ExecuteDeps) -> NeutronResult<Response<NeutronMsg>> {
    let receiver = NATIVE_ATOM_RECEIVER.load(deps.storage)?;
    let lper_native_token_balance = query_lper_balance(deps.as_ref(), &receiver.address)?;

    if lper_native_token_balance.amount.is_zero() {
        // The amount is zero, meaning we can dequeue from clock and move state to complete
        let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
        let clock_msg = covenant_clock::helpers::dequeue_msg(clock_addr.as_str())?;

        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;

        Ok(Response::default()
            .add_message(clock_msg)
            .add_attribute("method", "try_verify_lp")
            .add_attribute("status", "completed"))
    } else {
        // Balance is still there, so retry to send st token
        let ls_token_msg = try_send_ls_token(env, deps)?;

        Ok(Response::default()
            .add_submessage(ls_token_msg)
            .add_attribute("method", "try_verify_lp")
            .add_attribute("status", "retry_send_st_token"))
    }
}

/// tries to register an ICA on gaia on the connection stored in `NEUTRON_GAIA_CONNECTION_ID`
fn try_register_gaia_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let gaia_acc_id = INTERCHAIN_ACCOUNT_ID.to_string();
    let connection_id = NEUTRON_GAIA_CONNECTION_ID.load(deps.storage)?;
    let register = NeutronMsg::register_interchain_account(connection_id, gaia_acc_id.clone());

    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_gaia_ica")
        .add_message(register))
}

#[allow(unused)]
fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T>(
    deps: ExecuteDeps,
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::StAtomReceiver {} => {
            Ok(to_binary(&STRIDE_ATOM_RECEIVER.may_load(deps.storage)?)?)
        }
        QueryMsg::AtomReceiver {} => Ok(to_binary(&NATIVE_ATOM_RECEIVER.may_load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::DepositorInterchainAccountAddress {} => {
            query_depositor_interchain_address(deps, env)
        }
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
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::AutopilotFormat {} => Ok(to_binary(&AUTOPILOT_FORMAT.may_load(deps.storage)?)?),
    }
}

pub fn query_depositor_interchain_address(deps: QueryDeps, env: Env) -> NeutronResult<Binary> {
    let addr = get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID);

    match addr {
        Ok((addr, _)) => {
            let address_response = QueryInterchainAccountAddressResponse {
                interchain_account_address: addr,
            };
            Ok(to_binary(&address_response)?)
        }
        Err(_) => Err(NeutronError::Std(StdError::not_found("no ica stored"))),
    }
}

// returns ICA address from Neutron ICA SDK module
pub fn query_interchain_address(
    deps: QueryDeps,
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
    deps: QueryDeps,
    env: Env,
    interchain_account_id: String,
) -> NeutronResult<Binary> {
    Ok(to_binary(&get_ica(deps, &env, &interchain_account_id)?)?)
}

// returns the result
pub fn query_acknowledgement_result(
    deps: QueryDeps,
    env: Env,
    interchain_account_id: String,
    sequence_id: u64,
) -> NeutronResult<Binary> {
    let port_id = get_port_id(env.contract.address.as_str(), &interchain_account_id);
    let res = ACKNOWLEDGEMENT_RESULTS.may_load(deps.storage, (port_id, sequence_id))?;
    Ok(to_binary(&res)?)
}

pub fn query_errors_queue(deps: QueryDeps) -> NeutronResult<Binary> {
    let res = read_errors_from_queue(deps.storage)?;
    Ok(to_binary(&res)?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> StdResult<Response> {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: ExecuteDeps, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            st_atom_receiver,
            atom_receiver,
            gaia_neutron_ibc_transfer_channel_id,
            neutron_gaia_connection_id,
            gaia_stride_ibc_transfer_channel_id,
            ls_address,
            autopilot_format,
            ibc_fee,
            ibc_transfer_timeout,
            ica_timeout,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
                resp = resp.add_attribute("clock_addr", clock_addr);
            }

            if let Some(st_atom_receiver) = st_atom_receiver {
                STRIDE_ATOM_RECEIVER.save(deps.storage, &st_atom_receiver)?;
                resp = resp.add_attribute("ls_receiver_addr", st_atom_receiver.address);
                resp = resp.add_attribute("ls_receiver_amount", st_atom_receiver.amount);
            }

            if let Some(atom_receiver) = atom_receiver {
                NATIVE_ATOM_RECEIVER.save(deps.storage, &atom_receiver)?;
                resp = resp.add_attribute("native_receiver_addr", atom_receiver.address);
                resp = resp.add_attribute("native_receiver_amount", atom_receiver.amount);
            }

            if let Some(channel_id) = gaia_neutron_ibc_transfer_channel_id {
                GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID.save(deps.storage, &channel_id)?;
                resp = resp.add_attribute("gaia_neutron_ibc_transfer_channel_id", channel_id);
            }

            if let Some(connection_id) = neutron_gaia_connection_id {
                NEUTRON_GAIA_CONNECTION_ID.save(deps.storage, &connection_id)?;
                resp = resp.add_attribute("neutron_gaia_connection_id", connection_id);
            }

            if let Some(channel_id) = gaia_stride_ibc_transfer_channel_id {
                GAIA_STRIDE_IBC_TRANSFER_CHANNEL_ID.save(deps.storage, &channel_id)?;
                resp = resp.add_attribute("gaia_stride_ibc_transfer_channel_id", channel_id);
            }

            if let Some(ls_address) = ls_address {
                let addr = deps.api.addr_validate(&ls_address)?;
                LS_ADDRESS.save(deps.storage, &addr)?;
                resp = resp.add_attribute("ls_address", addr);
            }

            if let Some(autopilot_f) = autopilot_format {
                AUTOPILOT_FORMAT.save(deps.storage, &autopilot_f)?;
                resp = resp.add_attribute("autopilot_format", autopilot_f);
            }

            if let Some(timeout) = ibc_transfer_timeout {
                IBC_TRANSFER_TIMEOUT.save(deps.storage, &timeout)?;
                resp = resp.add_attribute("ibc_transfer_timeout", timeout);
            }

            if let Some(timeout) = ica_timeout {
                ICA_TIMEOUT.save(deps.storage, &timeout)?;
                resp = resp.add_attribute("ica_timeout", timeout);
            }

            if let Some(fee) = ibc_fee {
                if fee.ack_fee.is_empty() || fee.timeout_fee.is_empty() || !fee.recv_fee.is_empty() {
                    return Err(StdError::GenericErr {
                        msg: "invalid IbcFee".to_string(),
                    });
                }
                IBC_FEE.save(deps.storage, &fee)?;
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

// handler
fn sudo_open_ack(
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
        CONTRACT_STATE.save(deps.storage, &ContractState::ICACreated)?;
        return Ok(Response::default().add_attribute("method", "sudo_open_ack"));
    }
    Err(StdError::generic_err("Can't parse counterparty_version"))
}

fn sudo_response(deps: ExecuteDeps, request: RequestPacket, data: Binary) -> StdResult<Response> {
    let response = Response::default().add_attribute("method", "sudo_response");
    deps.api
        .debug(format!("WASMDEBUG: sudo_response: sudo received: {request:?} {data:?}").as_str());

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
        .debug(format!("WASMDEBUG: sudo_response: sudo payload: {payload:?}").as_str());

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
                deps.api
                    .debug(format!("MsgTransfer response: {:?}", item.data).as_str());
            }
            _ => {
                deps.api.debug(
                    format!("This type of acknowledgement is not implemented: {payload:?}")
                        .as_str(),
                );
            }
        }
    }

    if let Some(payload) = payload {
        // if payload.message == "try_send_funds" {
        //     CONTRACT_STATE.save(deps.storage, &ContractState::FundsSent)?;
        //     response = response.add_attribute("payload_message", "try_send_funds")
        // } else if payload.message == "try_receive_atom_from_ica" {
        //     CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
        //     response = response.add_attribute("payload_message", "try_receive_atom_from_ica")
        // }
        if payload.message == "try_send_native_token".to_string() {
            // we advance the state machine to validation phase where we will query the balances of
            // LP module to confirm that funds have arrived
            CONTRACT_STATE.save(deps.storage, &ContractState::VerifyNativeToken)?;
        }

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

    Ok(response)
}

fn sudo_timeout(deps: ExecuteDeps, _env: Env, request: RequestPacket) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo timeout request: {request:?}").as_str());

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

    // timeout means that the ICA channel is closed
    // we rollback the state to Instantiated to force reopen the channel
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    Ok(Response::default().add_attribute("method", "sudo_timeout"))
}

fn sudo_error(deps: ExecuteDeps, request: RequestPacket, details: String) -> StdResult<Response> {
    deps.api
        .debug(format!("WASMDEBUG: sudo error: {details}").as_str());
    deps.api
        .debug(format!("WASMDEBUG: request packet: {request:?}").as_str());

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

    Ok(Response::default().add_attribute("method", "sudo_error"))
}

// prepare_sudo_payload is called from reply handler
// The method is used to extract sequence id and channel from SubmitTxResponse to process sudo payload defined in msg_with_sudo_callback later in Sudo handler.
// Such flow msg_with_sudo_callback() -> reply() -> prepare_sudo_payload() -> sudo() allows you "attach" some payload to your SubmitTx message
// and process this payload when an acknowledgement for the SubmitTx message is received in Sudo handler
fn prepare_sudo_payload(mut deps: ExecuteDeps, _env: Env, msg: Reply) -> StdResult<Response> {
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

/// tries to retrieve the interchain account add
fn get_ica(
    deps: QueryDeps,
    env: &Env,
    interchain_account_id: &str,
) -> Result<(String, String), StdError> {
    let key = get_port_id(env.contract.address.as_str(), interchain_account_id);

    INTERCHAIN_ACCOUNTS
        .load(deps.storage, key)?
        .ok_or_else(|| StdError::generic_err("Interchain account is not created yet"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: ExecuteDeps, env: Env, msg: Reply) -> StdResult<Response> {
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
