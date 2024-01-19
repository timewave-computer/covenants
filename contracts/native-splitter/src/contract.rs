use std::collections::HashSet;

use cosmos_sdk_proto::cosmos::bank::v1beta1::{Input, MsgMultiSend, Output};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::traits::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, CosmosMsg, CustomQuery, Deps, DepsMut, Env, Fraction,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::neutron_ica::{get_default_ica_fee, RemoteChainInfo, SudoPayload};
use cw2::set_contract_version;
use neutron_sdk::bindings::types::ProtobufAny;
use neutron_sdk::interchain_txs::helpers::get_port_id;
use neutron_sdk::sudo::msg::SudoMsg;
use neutron_sdk::NeutronError;

use crate::msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    save_reply_payload, CLOCK_ADDRESS, CONTRACT_STATE, INTERCHAIN_ACCOUNTS, REMOTE_CHAIN_INFO,
    SPLIT_CONFIG_MAP, TRANSFER_AMOUNT,
};
use crate::sudo::{prepare_sudo_payload, sudo_error, sudo_open_ack, sudo_response, sudo_timeout};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    NeutronResult,
};

type QueryDeps<'a> = Deps<'a, NeutronQuery>;
type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

const INTERCHAIN_ACCOUNT_ID: &str = "rc-ica";
const CONTRACT_NAME: &str = "crates.io:covenant-native-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
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
    TRANSFER_AMOUNT.save(deps.storage, &msg.amount)?;

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
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "native_splitter_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attributes(remote_chain_info.get_response_attributes())
        .add_attributes(split_resp_attributes))
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

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    match CONTRACT_STATE.load(deps.storage)? {
        ContractState::Instantiated => try_register_ica(deps, env),
        ContractState::IcaCreated => try_split_funds(deps, env),
        ContractState::Completed => {
            Ok(Response::default().add_attribute("contract_state", "completed"))
        }
    }
}

fn try_register_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let register: NeutronMsg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string(),
        Some(vec![get_default_ica_fee()]),
    );
    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);

    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_ica")
        .add_message(register))
}

fn try_split_funds(mut deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;
    let amount = TRANSFER_AMOUNT.load(deps.storage)?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

            let splits =
                SPLIT_CONFIG_MAP.load(deps.storage, remote_chain_info.denom.to_string())?;

            let mut outputs: Vec<Output> = Vec::with_capacity(splits.len());
            for split_receiver in splits.iter() {
                // query the ibc forwarders for their ICA addresses
                // if either does not exist yet, error out
                let forwarder_deposit_address: Option<String> = deps.querier.query_wasm_smart(
                    split_receiver.addr.to_string(),
                    &covenant_utils::neutron_ica::CovenantQueryMsg::DepositAddress {},
                )?;

                let receiver_ica = match forwarder_deposit_address {
                    Some(ica) => ica,
                    None => {
                        return Err(NeutronError::Std(StdError::NotFound {
                            kind: "forwarder ica not created".to_string(),
                        }))
                    }
                };

                // get the fraction dedicated to this receiver
                let amt = amount
                    .checked_multiply_ratio(
                        split_receiver.share.numerator(),
                        split_receiver.share.denominator(),
                    )
                    .map_err(|e: cosmwasm_std::CheckedMultiplyRatioError| {
                        NeutronError::Std(StdError::GenericErr { msg: e.to_string() })
                    })?;

                let coin = Coin {
                    denom: remote_chain_info.denom.to_string(),
                    amount: amt.to_string(),
                };
                let output = Output {
                    address: receiver_ica,
                    coins: vec![coin.clone()],
                };

                outputs.push(output);
            }

            let mut inputs: Vec<Input> = Vec::new();
            let input = Input {
                address: address.to_string(),
                coins: vec![Coin {
                    denom: remote_chain_info.denom,
                    amount: amount.to_string(),
                }],
            };
            inputs.push(input);

            let multi_send_msg = MsgMultiSend { inputs, outputs };

            // Serialize the Delegate message.
            let mut buf = Vec::with_capacity(multi_send_msg.encoded_len());

            if let Err(e) = multi_send_msg.encode(&mut buf) {
                return Err(NeutronError::Std(StdError::generic_err(format!(
                    "Encode error: {}",
                    e
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
                remote_chain_info.ibc_fee,
            );
            let sudo_msg = msg_with_sudo_callback(
                deps.branch(),
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "split_funds_msg".to_string(),
                },
            )?;
            Ok(Response::default().add_submessages(vec![sudo_msg]))
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

fn query_deposit_address(deps: QueryDeps, env: Env) -> Result<Option<String>, StdError> {
    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    /*
       here we cover three possible cases:
       - 1. ICA had been created -> nice
       - 2. ICA creation request had been submitted but did not receive
           the channel_open_ack yet -> None
       - 3. ICA creation request hadn't been submitted yet -> None
    */
    INTERCHAIN_ACCOUNTS
        .may_load(deps.storage, key)
        .map(|entry| entry.flatten().map(|x| x.0))
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
