use std::collections::BTreeSet;

use cosmos_sdk_proto::cosmos::bank::v1beta1::{Input, MsgMultiSend, Output};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, Uint128,
};
use covenant_utils::{
    ica::{
        get_ica, msg_with_sudo_callback, prepare_sudo_payload, query_ica_registration_fee,
        sudo_error, sudo_open_ack, sudo_response, sudo_timeout, INTERCHAIN_ACCOUNT_ID,
    },
    neutron::{
        assert_ibc_fee_coverage, get_proto_coin, query_ibc_fee, to_proto_msg_transfer,
        RemoteChainInfo, SudoPayload,
    },
    op_mode::{verify_caller, ContractOperationMode},
};
use cw2::set_contract_version;
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery, types::ProtobufAny},
    interchain_txs::helpers::get_port_id,
    sudo::msg::SudoMsg,
    NeutronError, NeutronResult,
};
use prost::Message;

use crate::state::{IbcForwarderIcaStateHelper, FALLBACK_ADDRESS};
use crate::{error::ContractError, msg::FallbackAddressUpdateConfig};
use crate::{
    helpers::{get_next_memo, MsgTransfer},
    msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CONTRACT_OP_MODE, CONTRACT_STATE, INTERCHAIN_ACCOUNTS, NEXT_CONTRACT, REMOTE_CHAIN_INFO,
        TRANSFER_AMOUNT,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SUDO_PAYLOAD_REPLY_ID: u64 = 1;

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

    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    let op_mode = ContractOperationMode::try_init(deps.api, msg.op_mode_cfg.clone())?;

    CONTRACT_OP_MODE.save(deps.storage, &op_mode)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    TRANSFER_AMOUNT.save(deps.storage, &msg.amount)?;
    let remote_chain_info = RemoteChainInfo {
        connection_id: msg.remote_chain_connection_id.to_string(),
        channel_id: msg.remote_chain_channel_id.to_string(),
        denom: msg.denom.to_string(),
        ica_timeout: msg.ica_timeout,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
    };
    REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    if let Some(addr) = &msg.fallback_address {
        FALLBACK_ADDRESS.save(deps.storage, addr)?;
    }

    Ok(Response::default()
        .add_attribute("method", "ibc_forwarder_instantiate")
        .add_attribute("next_contract", next_contract)
        .add_attribute("contract_state", "instantiated")
        .add_attributes(msg.get_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::DistributeFallback { coins } => try_distribute_fallback(deps, env, info, coins),
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

fn try_distribute_fallback(
    mut deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    coins: Vec<cosmwasm_std::Coin>,
) -> NeutronResult<Response<NeutronMsg>> {
    // load the fallback address or error out if its not set
    let destination = match FALLBACK_ADDRESS.may_load(deps.storage)? {
        Some(addr) => addr,
        None => return Err(ContractError::MissingFallbackAddress {}.into()),
    };
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

    let min_ibc_fee_config = query_ibc_fee(deps.querier)?;
    assert_ibc_fee_coverage(info, min_ibc_fee_config.total_ntrn_fee, Uint128::one())?;

    // we iterate over coins to be distributed, validate them, and generate the proto coins to be sent
    let mut encountered_denoms: BTreeSet<String> = BTreeSet::new();
    let mut proto_coins: Vec<cosmos_sdk_proto::cosmos::base::v1beta1::Coin> = vec![];

    for coin in coins {
        // validate that target denom is not passed for fallback distribution
        ensure!(
            coin.denom != remote_chain_info.denom,
            Into::<NeutronError>::into(ContractError::UnauthorizedDenomDistribution {})
        );

        // error out if denom is duplicated
        ensure!(
            encountered_denoms.insert(coin.denom.to_string()),
            Into::<NeutronError>::into(ContractError::DuplicateDenomDistribution {})
        );

        proto_coins.push(get_proto_coin(coin.denom, coin.amount));
    }

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.may_load(deps.storage, port_id.clone())?;
    if let Some(Some((address, controller_conn_id))) = interchain_account {
        let multi_send_msg = MsgMultiSend {
            inputs: vec![Input {
                address,
                coins: proto_coins.clone(),
            }],
            outputs: vec![Output {
                address: destination,
                coins: proto_coins,
            }],
        };

        // Serialize the multi send message.
        let mut buf = Vec::with_capacity(multi_send_msg.encoded_len());

        if let Err(e) = multi_send_msg.encode(&mut buf) {
            return Err(NeutronError::Std(StdError::generic_err(format!(
                "Encode error: {e:}",
            ))));
        }

        let any_msg = ProtobufAny {
            type_url: "/cosmos.bank.v1beta1.MsgMultiSend".to_string(),
            value: Binary::from(buf),
        };
        let submit_msg = NeutronMsg::submit_tx(
            controller_conn_id,
            INTERCHAIN_ACCOUNT_ID.to_string(),
            vec![any_msg],
            "".to_string(),
            remote_chain_info.ica_timeout.u64(),
            min_ibc_fee_config.ibc_fee,
        );
        let state_helper = IbcForwarderIcaStateHelper;
        let sudo_msg = msg_with_sudo_callback(
            &state_helper,
            deps.branch(),
            submit_msg,
            SudoPayload {
                port_id,
                message: "distribute_fallback_multisend".to_string(),
            },
            SUDO_PAYLOAD_REPLY_ID,
        )?;

        Ok(Response::default()
            .add_attribute("method", "try_forward_fallback")
            .add_submessages(vec![sudo_msg]))
    } else {
        Err(NeutronError::Std(StdError::generic_err("no ica found")))
    }
}

/// attempts to advance the state machine. validates the caller to be the clock.
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    verify_caller(&info.sender, &CONTRACT_OP_MODE.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_register_ica(deps, env),
        ContractState::IcaCreated => try_forward_funds(env, deps),
    }
}

/// tries to register an ICA on the remote chain
fn try_register_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let ica_registration_fee = query_ica_registration_fee(deps.querier)?;

    let register_msg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string(),
        Some(ica_registration_fee),
    );

    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);

    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_ica")
        .add_message(register_msg))
}

fn try_forward_funds(env: Env, mut deps: ExecuteDeps) -> NeutronResult<Response<NeutronMsg>> {
    // first we verify whether the next contract is ready for receiving the funds
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query: Option<String> = deps.querier.query_wasm_smart(
        next_contract.to_string(),
        &covenant_utils::neutron::QueryMsg::DepositAddress {},
    )?;

    // if query returns None, then we error and wait
    let Some(deposit_address) = deposit_address_query else {
        return Err(NeutronError::Std(StdError::not_found(
            "Next contract is not ready for receiving the funds yet",
        )));
    };

    let min_fee_query_response = query_ibc_fee(deps.querier)?;

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
            let amount = TRANSFER_AMOUNT.load(deps.storage)?;

            let memo = get_next_memo(deps.querier, next_contract.as_str())?;

            let transfer_msg = MsgTransfer {
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
                memo,
            };

            let protobuf_msg = to_proto_msg_transfer(transfer_msg)?;

            // tx to our ICA that wraps the transfer message defined above
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![protobuf_msg],
                "".to_string(),
                remote_chain_info.ica_timeout.u64(),
                min_fee_query_response.ibc_fee,
            );

            // sudo callback msg
            // let state_helper = IbcForwarderIcaStateHelper;
            let submsg = msg_with_sudo_callback(
                &IbcForwarderIcaStateHelper,
                deps.branch(),
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "try_forward_funds".to_string(),
                },
                SUDO_PAYLOAD_REPLY_ID,
            )?;

            Ok(Response::default()
                .add_attribute("method", "try_forward_funds")
                .add_submessage(submsg))
        }
        None => {
            // I can't think of a case of how we could end up here as `sudo_open_ack`
            // callback advances the state to `ICACreated` and stores the ICA.
            // just in case, we revert the state to `Instantiated` to restart the flow.
            CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
            Ok(Response::default()
                .add_attribute("method", "try_forward_funds")
                .add_attribute("error", "no_ica_found"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        // we expect to receive funds into our ICA account on the remote chain.
        // if the ICA had not been opened yet, we return `None` so that the
        // contract querying this will be instructed to wait and retry.
        QueryMsg::DepositAddress {} => {
            let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
            // here we want to return None instead of any errors in case no ICA
            // is registered yet
            let ica = match INTERCHAIN_ACCOUNTS.may_load(deps.storage, key)? {
                Some(entry) => {
                    if let Some((addr, _)) = entry {
                        Some(addr)
                    } else {
                        None
                    }
                }
                None => None,
            };

            Ok(to_json_binary(&ica)?)
        }
        QueryMsg::IcaAddress {} => Ok(to_json_binary(
            &get_ica(
                &IbcForwarderIcaStateHelper,
                deps.storage,
                env.contract.address.as_str(),
                INTERCHAIN_ACCOUNT_ID,
            )?
            .0,
        )?),
        QueryMsg::RemoteChainInfo {} => {
            Ok(to_json_binary(&REMOTE_CHAIN_INFO.may_load(deps.storage)?)?)
        }
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::FallbackAddress {} => {
            Ok(to_json_binary(&FALLBACK_ADDRESS.may_load(deps.storage)?)?)
        }
        QueryMsg::OperationMode {} => {
            Ok(to_json_binary(&CONTRACT_OP_MODE.may_load(deps.storage)?)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        // For handling successful (non-error) acknowledgements.
        SudoMsg::Response { request, data } => sudo_response(request, data),

        // For handling error acknowledgements.
        SudoMsg::Error { request, details } => sudo_error(request, details),

        // For handling error timeouts.
        SudoMsg::Timeout { request } => {
            sudo_timeout(&IbcForwarderIcaStateHelper, deps, env, request)
        }

        // For handling successful registering of ICA
        SudoMsg::OpenAck {
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        } => sudo_open_ack(
            &IbcForwarderIcaStateHelper,
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
pub fn reply(deps: ExecuteDeps, env: Env, msg: Reply) -> StdResult<Response<NeutronMsg>> {
    match msg.id {
        SUDO_PAYLOAD_REPLY_ID => prepare_sudo_payload(&IbcForwarderIcaStateHelper, deps, env, msg),
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
            op_mode,
            next_contract,
            remote_chain_info,
            transfer_amount,
            fallback_address,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(op_mode_cfg) = op_mode {
                let updated_op_mode = ContractOperationMode::try_init(deps.api, op_mode_cfg)
                    .map_err(|err| StdError::generic_err(err.to_string()))?;

                CONTRACT_OP_MODE.save(deps.storage, &updated_op_mode)?;
                resp = resp.add_attribute("op_mode", format!("{:?}", updated_op_mode));
            }

            if let Some(addr) = next_contract {
                let next_contract_addr = deps.api.addr_validate(&addr)?;
                NEXT_CONTRACT.save(deps.storage, &next_contract_addr)?;
                resp = resp.add_attribute("next_contract", addr);
            }

            if let Some(rci) = *remote_chain_info {
                REMOTE_CHAIN_INFO.save(deps.storage, &rci)?;
                resp = resp.add_attributes(rci.get_response_attributes());
            }

            if let Some(amount) = transfer_amount {
                TRANSFER_AMOUNT.save(deps.storage, &amount)?;
                resp = resp.add_attribute("transfer_amount", amount.to_string());
            }

            if let Some(config) = fallback_address {
                match config {
                    FallbackAddressUpdateConfig::ExplicitAddress(addr) => {
                        FALLBACK_ADDRESS.save(deps.storage, &addr)?;
                        resp = resp.add_attribute("fallback_address", addr);
                    }
                    FallbackAddressUpdateConfig::Disable {} => {
                        FALLBACK_ADDRESS.remove(deps.storage);
                        resp = resp.add_attribute("fallback_address", "removed");
                    }
                }
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
