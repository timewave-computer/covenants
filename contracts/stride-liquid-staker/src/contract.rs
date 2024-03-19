use cosmos_sdk_proto::ibc::applications::transfer::v1::MsgTransfer;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, to_json_binary, to_json_string, Binary, Coin, CosmosMsg, CustomQuery, Deps, DepsMut,
    Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::neutron::{self, get_proto_coin, OpenAckVersion, RemoteChainInfo, SudoPayload};
use cw2::set_contract_version;
use neutron_sdk::query::min_ibc_fee::MinIbcFeeResponse;

use crate::helpers::{Autopilot, AutopilotConfig};
use crate::msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    read_reply_payload, save_reply_payload, save_sudo_payload, CLOCK_ADDRESS, CONTRACT_STATE,
    INTERCHAIN_ACCOUNTS, NEXT_CONTRACT, REMOTE_CHAIN_INFO,
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

const CONTRACT_NAME: &str = "crates.io:covenant-stride-liquid-staker";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

type QueryDeps<'a> = Deps<'a, NeutronQuery>;
type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate the addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let next_contract = deps.api.addr_validate(&msg.next_contract)?;

    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    let remote_chain_info = RemoteChainInfo {
        connection_id: msg.neutron_stride_ibc_connection_id,
        channel_id: msg.stride_neutron_ibc_transfer_channel_id,
        denom: msg.ls_denom,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
        ica_timeout: msg.ica_timeout,
    };
    REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "ls_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attribute("next_contract", next_contract)
        .add_attributes(remote_chain_info.get_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Transfer { amount } => {
            let ica_address = get_ica(deps.as_ref(), &env, INTERCHAIN_ACCOUNT_ID);
            match ica_address {
                Ok(_) => try_execute_transfer(deps, env, info, amount),
                Err(_) => Ok(Response::default()
                    .add_attribute("method", "try_permisionless_transfer")
                    .add_attribute("ica_status", "not_created")),
            }
        }
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_register_stride_ica(deps, env),
        ContractState::IcaCreated => Ok(Response::default()),
    }
}

/// registers an interchain account on stride with port_id associated with `INTERCHAIN_ACCOUNT_ID`
fn try_register_stride_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let register_fee: Option<Vec<Coin>> = Some(coins(1000001, "untrn"));
    let register: NeutronMsg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string(),
        register_fee,
    );
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
    deps: ExecuteDeps,
    env: Env,
    _info: MessageInfo,
    amount: Uint128,
) -> NeutronResult<Response<NeutronMsg>> {
    // first we verify whether the next contract is ready for receiving the funds
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query = deps
        .querier
        .query_wasm_smart(next_contract, &neutron::QueryMsg::DepositAddress {})?;

    // if query returns None, then we error and wait
    let Some(deposit_address) = deposit_address_query else {
        return Err(NeutronError::Std(StdError::not_found(
            "Next contract is not ready for receiving the funds yet",
        )));
    };

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;
    let min_fee_query_response: MinIbcFeeResponse =
        deps.querier.query(&NeutronQuery::MinIbcFee {}.into())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

            // inner MsgTransfer that will be sent from stride to neutron.
            // because of this message delivery depending on the ica wrapper below,
            // timeout_timestamp = current block + ica timeout + ibc_transfer_timeout
            let msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: remote_chain_info.channel_id,
                token: Some(get_proto_coin(remote_chain_info.denom, amount)),
                sender: address,
                receiver: deposit_address,
                timeout_height: None,
                timeout_timestamp: env
                    .block
                    .time
                    .plus_seconds(remote_chain_info.ica_timeout.u64())
                    .plus_seconds(remote_chain_info.ibc_transfer_timeout.u64())
                    .nanos(),
            };

            let protobuf = neutron::to_proto_msg_transfer(msg)?;

            // wrap the protobuf of MsgTransfer into a message to be executed
            // by our interchain account
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![protobuf],
                "".to_string(),
                remote_chain_info.ica_timeout.u64(),
                min_fee_query_response.min_fee,
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
                .add_attribute("method", "try_execute_transfer"))
        }
        None => {
            // I can't think of a case of how we could end up here as `sudo_open_ack`
            // callback advances the state to `ICACreated` and stores the ICA.
            // just in case, we revert the state to `Instantiated` to restart the flow.
            CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
            Ok(Response::default()
                .add_attribute("method", "try_execute_transfer")
                .add_attribute("error", "no_ica_found"))
        }
    }
}

fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T>(
    deps: ExecuteDeps,
    msg: C,
    payload: SudoPayload,
) -> StdResult<SubMsg<T>> {
    save_reply_payload(deps.storage, payload)?;
    Ok(SubMsg::reply_on_success(msg, SUDO_PAYLOAD_REPLY_ID))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::IcaAddress {} => Ok(to_json_binary(
            &get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID)?.0,
        )?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => {
            let ica = get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID)?.0;

            let autopilot = Autopilot {
                autopilot: AutopilotConfig {
                    receiver: ica.to_string(),
                    stakeibc: crate::helpers::Stakeibc {
                        action: "LiquidStake".to_string(),
                        stride_address: ica,
                    },
                },
            };

            let autopilot_str = to_json_string(&autopilot)?;

            Ok(to_json_binary(&autopilot_str)?)
        }
        QueryMsg::RemoteChainInfo {} => {
            Ok(to_json_binary(&REMOTE_CHAIN_INFO.may_load(deps.storage)?)?)
        }
        QueryMsg::NextMemo {} => {
            // 1. receiver = query ICA
            let ica = get_ica(deps, &env, INTERCHAIN_ACCOUNT_ID)?.0;

            let autopilot = Autopilot {
                autopilot: AutopilotConfig {
                    receiver: ica.to_string(),
                    stakeibc: crate::helpers::Stakeibc {
                        action: "LiquidStake".to_string(),
                        stride_address: ica,
                    },
                },
            };

            let autopilot_str = to_json_string(&autopilot)?;

            Ok(to_json_binary(&autopilot_str)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> Result<Response<NeutronMsg>, StdError> {
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

fn sudo_response(
    _deps: ExecuteDeps,
    request: RequestPacket,
    _data: Binary,
) -> StdResult<Response<NeutronMsg>> {
    // either of these errors will close the channel
    request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    request
        .source_channel
        .ok_or_else(|| StdError::generic_err("channel_id not found"))?;

    Ok(Response::default().add_attribute("method", "sudo_response"))
}

fn sudo_timeout(
    deps: ExecuteDeps,
    _env: Env,
    _request: RequestPacket,
) -> StdResult<Response<NeutronMsg>> {
    // revert the state to Instantiated to force re-creation of ICA
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // returning Ok as this is anticipated. channel is already closed.
    Ok(Response::default())
}

fn sudo_error(
    _deps: ExecuteDeps,
    request: RequestPacket,
    _details: String,
) -> StdResult<Response<NeutronMsg>> {
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
fn prepare_sudo_payload(
    mut deps: ExecuteDeps,
    _env: Env,
    msg: Reply,
) -> StdResult<Response<NeutronMsg>> {
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
pub fn reply(deps: ExecuteDeps, env: Env, msg: Reply) -> StdResult<Response<NeutronMsg>> {
    match msg.id {
        SUDO_PAYLOAD_REPLY_ID => prepare_sudo_payload(deps, env, msg),
        _ => Err(StdError::generic_err(format!(
            "unsupported reply message id {}",
            msg.id
        ))),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: ExecuteDeps, _env: Env, msg: MigrateMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            next_contract,
            remote_chain_info,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(addr) = clock_addr {
                let addr = deps.api.addr_validate(&addr)?;
                CLOCK_ADDRESS.save(deps.storage, &addr)?;
                resp = resp.add_attribute("clock_addr", addr.to_string());
            }

            if let Some(addr) = next_contract {
                let addr = deps.api.addr_validate(&addr)?;
                resp = resp.add_attribute("next_contract", addr.to_string());
                NEXT_CONTRACT.save(deps.storage, &addr)?;
            }

            if let Some(rci) = remote_chain_info {
                REMOTE_CHAIN_INFO.save(deps.storage, &rci)?;
                resp = resp.add_attributes(rci.get_response_attributes());
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
