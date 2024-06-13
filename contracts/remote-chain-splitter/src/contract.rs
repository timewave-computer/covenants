use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

use cosmos_sdk_proto::cosmos::bank::v1beta1::{Input, MsgMultiSend, Output};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::traits::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Attribute, Binary, Deps, DepsMut, Env, Fraction, MessageInfo, Reply,
    Response, StdError, StdResult, Uint128,
};
use covenant_utils::ica::{
    get_ica, msg_with_sudo_callback, prepare_sudo_payload, query_ica_registration_fee, sudo_error,
    sudo_open_ack, sudo_response, sudo_timeout, INTERCHAIN_ACCOUNT_ID,
};
use covenant_utils::neutron::{
    assert_ibc_fee_coverage, get_proto_coin, query_ibc_fee, RemoteChainInfo, SudoPayload,
};
use covenant_utils::op_mode::{verify_caller, ContractOperationMode};
use covenant_utils::{neutron, soft_validate_remote_chain_addr};
use cw2::set_contract_version;
use neutron_sdk::bindings::types::ProtobufAny;
use neutron_sdk::interchain_txs::helpers::get_port_id;
use neutron_sdk::query::min_ibc_fee::MinIbcFeeResponse;
use neutron_sdk::sudo::msg::SudoMsg;
use neutron_sdk::NeutronError;

use crate::error::ContractError;
use crate::msg::{
    ContractState, ExecuteMsg, FallbackAddressUpdateConfig, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{
    RemoteChainSplitteIcaStateHelper, CONTRACT_OP_MODE, CONTRACT_STATE, FALLBACK_ADDRESS,
    INTERCHAIN_ACCOUNTS, REMOTE_CHAIN_INFO, SPLIT_CONFIG_MAP, TRANSFER_AMOUNT,
};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    NeutronResult,
};

type QueryDeps<'a> = Deps<'a, NeutronQuery>;
type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let op_mode = ContractOperationMode::try_init(deps.api, msg.op_mode_cfg.clone())?;
    CONTRACT_OP_MODE.save(deps.storage, &op_mode)?;

    let remote_chain_info = RemoteChainInfo {
        connection_id: msg.remote_chain_connection_id,
        channel_id: msg.remote_chain_channel_id,
        denom: msg.denom,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
        ica_timeout: msg.ica_timeout,
    };
    REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    TRANSFER_AMOUNT.save(deps.storage, &msg.amount)?;
    if let Some(addr) = &msg.fallback_address {
        soft_validate_remote_chain_addr(deps.api, addr)?;
        FALLBACK_ADDRESS.save(deps.storage, addr)?;
    }

    // validate each split and store it in a map
    let mut split_resp_attributes: Vec<Attribute> = Vec::with_capacity(msg.splits.len());

    for (denom, split_config) in msg.splits {
        split_config.validate_shares_and_receiver_addresses(deps.api)?;
        split_resp_attributes.push(split_config.get_response_attribute(denom.to_string()));
        SPLIT_CONFIG_MAP.save(deps.storage, denom, &split_config)?;
    }

    Ok(Response::default()
        .add_attribute("method", "remote_chain_splitter_instantiate")
        .add_attribute("op_mode", format!("{:?}", op_mode))
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
    match (msg, CONTRACT_STATE.load(deps.storage)?) {
        // if the contract is in the instantiated state, we try to register the ICA
        (ExecuteMsg::Tick {}, ContractState::Instantiated) => {
            verify_caller(&info.sender, &CONTRACT_OP_MODE.load(deps.storage)?)?;
            try_register_ica(deps, env)
        },
        // if the contract is in the IcaCreated state, we try to split the funds
        (ExecuteMsg::Tick {}, ContractState::IcaCreated) => {
            verify_caller(&info.sender, &CONTRACT_OP_MODE.load(deps.storage)?)?;
            try_split_funds(deps, env)
        },
        // in order to distribute the fallback split, ICA needs to be created
        (ExecuteMsg::DistributeFallback {..}, ContractState::Instantiated) => {
            Err(StdError::generic_err("no ica found").into())
        },
        // if the contract is in the IcaCreated state, we try to distribute the fallback split
        (ExecuteMsg::DistributeFallback { coins }, ContractState::IcaCreated) => {
            try_distribute_fallback(deps, env, info, coins)
        },
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
    let ibc_fee_response = query_ibc_fee(deps.querier)?;

    assert_ibc_fee_coverage(info, ibc_fee_response.total_ntrn_fee, Uint128::one())?;

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
    let interchain_account = INTERCHAIN_ACCOUNTS.load(deps.storage, port_id.clone())?;
    if let Some((address, controller_conn_id)) = interchain_account {
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
            ibc_fee_response.ibc_fee,
        );
        let sudo_msg = msg_with_sudo_callback(
            &RemoteChainSplitteIcaStateHelper,
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

fn try_register_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    let ica_registration_fee = query_ica_registration_fee(deps.querier)?;

    let register: NeutronMsg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string(),
        Some(ica_registration_fee),
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
    let min_fee_query_response: MinIbcFeeResponse =
        deps.querier.query(&NeutronQuery::MinIbcFee {}.into())?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

            let splits = SPLIT_CONFIG_MAP
                .load(deps.storage, remote_chain_info.denom.to_string())?
                .receivers;

            let mut outputs: Vec<Output> = Vec::with_capacity(splits.len());
            let mut total_allocated = Uint128::zero();
            for (split_receiver, share) in splits.iter() {
                // query the ibc forwarders for their ICA addresses
                // if either does not exist yet, error out
                let forwarder_deposit_address: Option<String> = deps.querier.query_wasm_smart(
                    split_receiver.to_string(),
                    &neutron::CovenantQueryMsg::DepositAddress {},
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
                    .checked_multiply_ratio(share.numerator(), share.denominator())
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
                total_allocated += amt;
                outputs.push(output);
            }

            // if there is no leftover, nothing happens.
            // otherwise we add the leftover to the first receiver.
            if let Some(output) = outputs.first_mut() {
                output.coins[0].amount = (Uint128::from_str(&output.coins[0].amount)?
                    + (amount - total_allocated))
                    .to_string();
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

            // Serialize the multi send message.
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
                min_fee_query_response.min_fee,
            );
            let sudo_msg = msg_with_sudo_callback(
                &RemoteChainSplitteIcaStateHelper,
                deps.branch(),
                submit_msg,
                SudoPayload {
                    port_id,
                    message: "split_funds_msg".to_string(),
                },
                SUDO_PAYLOAD_REPLY_ID,
            )?;
            Ok(Response::default()
                .add_attribute("method", "try_split_funds")
                .add_submessages(vec![sudo_msg]))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
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
            &get_ica(
                &RemoteChainSplitteIcaStateHelper,
                deps.storage,
                env.contract.address.as_str(),
                INTERCHAIN_ACCOUNT_ID,
            )?
            .0,
        )?),
        QueryMsg::FallbackAddress {} => {
            Ok(to_json_binary(&FALLBACK_ADDRESS.may_load(deps.storage)?)?)
        }
        QueryMsg::OperationMode {} => {
            Ok(to_json_binary(&CONTRACT_OP_MODE.may_load(deps.storage)?)?)
        }
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        // For handling successful (non-error) acknowledgements.
        SudoMsg::Response { request, data } => sudo_response(request, data),
        // For handling error acknowledgements.
        SudoMsg::Error { request, details } => sudo_error(request, details),
        // For handling error timeouts.
        SudoMsg::Timeout { request } => {
            sudo_timeout(&RemoteChainSplitteIcaStateHelper, deps, env, request)
        }
        // For handling successful registering of ICA
        SudoMsg::OpenAck {
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        } => sudo_open_ack(
            &RemoteChainSplitteIcaStateHelper,
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
pub fn migrate(deps: ExecuteDeps, _env: Env, msg: MigrateMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        MigrateMsg::UpdateConfig {
            op_mode,
            remote_chain_info,
            splits,
            fallback_address,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(op_mode_cfg) = op_mode {
                let updated_op_mode = ContractOperationMode::try_init(deps.api, op_mode_cfg)
                    .map_err(|err| StdError::generic_err(err.to_string()))?;

                CONTRACT_OP_MODE.save(deps.storage, &updated_op_mode)?;
                resp = resp.add_attribute("op_mode", format!("{:?}", updated_op_mode));
            }

            if let Some(remote_chain_info) = remote_chain_info {
                REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
                resp = resp.add_attribute("remote_chain_info", format!("{remote_chain_info:?}"));
            }

            if let Some(splits) = splits {
                let mut split_resp_attributes: Vec<Attribute> = Vec::with_capacity(splits.len());
                let mut encountered_denoms: HashSet<String> = HashSet::with_capacity(splits.len());

                for (denom, split) in splits {
                    // if denom had not yet been encountered we proceed, otherwise error
                    if encountered_denoms.insert(denom.to_string()) {
                        split.validate_shares_and_receiver_addresses(deps.api)?;
                        split_resp_attributes.push(split.get_response_attribute(denom.to_string()));
                        SPLIT_CONFIG_MAP.save(deps.storage, denom.to_string(), &split)?;

                        resp = resp.add_attribute(
                            format!("split-{}", denom),
                            format!("{:?}", split.receivers),
                        );
                    } else {
                        return Err(StdError::generic_err(format!(
                            "multiple {:?} entries",
                            denom
                        )));
                    }
                }
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: ExecuteDeps, env: Env, msg: Reply) -> StdResult<Response<NeutronMsg>> {
    match msg.id {
        SUDO_PAYLOAD_REPLY_ID => {
            prepare_sudo_payload(&RemoteChainSplitteIcaStateHelper, deps, env, msg)
        }
        _ => Err(StdError::generic_err(format!(
            "unsupported reply message id {}",
            msg.id
        ))),
    }
}
