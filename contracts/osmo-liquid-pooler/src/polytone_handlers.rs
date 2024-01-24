use std::str::FromStr;

use cosmwasm_std::{
    coin, from_json, to_json_binary, Addr, Binary, Coin, CosmosMsg, DepsMut, Empty, Env, IbcMsg,
    IbcTimeout, MessageInfo, QueryRequest, Response, StdResult, Uint128, Uint64, WasmMsg,
};
use covenant_utils::{
    get_polytone_execute_msg_binary, get_polytone_query_msg_binary, query_polytone_proxy_address,
    withdraw_lp_helper::WithdrawLPMsgs,
};
use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

use crate::{
    contract::{
        CREATE_PROXY_CALLBACK_ID, PROVIDE_LIQUIDITY_CALLBACK_ID, PROXY_BALANCES_QUERY_CALLBACK_ID,
        WITHDRAW_LIQUIDITY_BALANCES_QUERY_CALLBACK_ID, WITHDRAW_LIQUIDITY_CALLBACK_ID,
    },
    error::ContractError,
    msg::{ContractState, IbcConfig, LiquidityProvisionConfig},
    state::{
        CONTRACT_STATE, HOLDER_ADDRESS, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS,
        POLYTONE_CALLBACKS, PROXY_ADDRESS,
    },
};

use polytone::callbacks::{
    Callback as PolytoneCallback, CallbackMessage, CallbackRequest, ErrorResponse,
    ExecutionResponse,
};

/// attempts to advance the state machine. performs `info.sender` validation.
pub fn try_handle_callback(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    msg: CallbackMessage,
) -> NeutronResult<Response<NeutronMsg>> {
    // only the note can submit a callback
    if info.sender != NOTE_ADDRESS.load(deps.storage)? {
        return Err(ContractError::Unauthorized {}.to_neutron_std());
    }

    match msg.result {
        PolytoneCallback::Query(resp) => process_query_callback(env, deps, resp, msg.initiator_msg),
        PolytoneCallback::Execute(resp) => {
            process_execute_callback(env, deps, resp, msg.initiator_msg)
        }
        PolytoneCallback::FatalError(resp) => process_fatal_error_callback(env, deps, resp),
    }
}

fn process_query_callback(
    env: Env,
    deps: DepsMut,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    // decode the initiator message callback id into u8
    let initiator_msg: u8 = from_json(initiator_msg)?;

    match initiator_msg {
        PROXY_BALANCES_QUERY_CALLBACK_ID => {
            handle_proxy_balances_callback(deps, env, query_callback_result)
        }
        WITHDRAW_LIQUIDITY_BALANCES_QUERY_CALLBACK_ID => {
            handle_withdraw_liquidity_proxy_balances_callback(deps, env, query_callback_result)
        }
        _ => Err(ContractError::PolytoneError(format!(
            "unexpected callback id: {:?}",
            initiator_msg
        ))
        .to_neutron_std()),
    }
}

fn process_execute_callback(
    env: Env,
    deps: DepsMut,
    execute_callback_result: Result<ExecutionResponse, String>,
    initiator_msg: Binary,
) -> NeutronResult<Response<NeutronMsg>> {
    let initiator_msg: u8 = from_json(initiator_msg)?;
    let callback_result: ExecutionResponse = match execute_callback_result {
        Ok(val) => val,
        Err(e) => return Err(ContractError::PolytoneError(e).to_neutron_std()),
    };

    match initiator_msg {
        PROVIDE_LIQUIDITY_CALLBACK_ID => {
            POLYTONE_CALLBACKS.save(
                deps.storage,
                format!(
                    "provide_liquidity_callback : {:?}",
                    env.block.height.to_string()
                ),
                &to_json_binary(&callback_result)?.to_string(),
            )?;

            for submsg_response in callback_result.result {
                if submsg_response.data.is_some() {
                    if let Some(response_binary) = submsg_response.data {
                        POLYTONE_CALLBACKS.save(
                            deps.storage,
                            response_binary.to_string(),
                            &response_binary.to_base64(),
                        )?;
                    }
                    // todo: do we want to do something here?
                }
            }
        }
        CREATE_PROXY_CALLBACK_ID => {
            let note_address = NOTE_ADDRESS.load(deps.storage)?;

            let proxy_address = query_polytone_proxy_address(
                env.contract.address.to_string(),
                note_address.to_string(),
                deps.querier,
            )?;
            // result contains nothing
            POLYTONE_CALLBACKS.save(
                deps.storage,
                format!("create_proxy_callback : {:?}", env.block.height.to_string()),
                &to_json_binary(&callback_result)?.to_string(),
            )?;
            if let Some(addr) = proxy_address {
                PROXY_ADDRESS.save(deps.storage, &addr)?;
                CONTRACT_STATE.update(deps.storage, |state| -> StdResult<_> {
                    // little sanity check. should not end up in catchall arm,
                    // but if for some reason we receive a proxy created callback
                    // when we are not in Instantiated state, we do not update
                    match state {
                        ContractState::Instantiated => Ok(ContractState::ProxyCreated),
                        _ => Ok(state),
                    }
                })?;
            }
        }
        WITHDRAW_LIQUIDITY_CALLBACK_ID => {
            // can try to decode the response attribute here
            // callback_result.result[0] contains the events
            // query the events for one that has "type" == "wasm"
            // and search its attributes for one where key == "refund_tokens".
            // type is polytone ExecutionResponse
            // let refund_coins: Vec<Coin> = from_json(value)?;
            for callback_response in callback_result.clone().result {
                for event in callback_response.events {
                    if event.ty == "wasm" {
                        for attr in event.attributes {
                            if attr.key == "refund_tokens" {
                                let refunded_coins: Vec<Coin> = match from_json(&attr.value) {
                                    Ok(coins) => coins,
                                    Err(e) => {
                                        POLYTONE_CALLBACKS.save(
                                            deps.storage,
                                            format!(
                                                "withdraw_liquidity_callback_REFUND_TOKENS_error : {:?}",
                                                env.block.height.to_string()
                                            ),
                                            &e.to_string(),
                                        )?;
                                        vec![]
                                    }
                                };

                                CONTRACT_STATE.save(
                                    deps.storage,
                                    &ContractState::Distributing {
                                        coins: refunded_coins,
                                    },
                                )?;

                                POLYTONE_CALLBACKS.save(
                                    deps.storage,
                                    format!(
                                        "withdraw_liquidity_callback_REFUND_TOKENS : {:?}",
                                        env.block.height.to_string()
                                    ),
                                    &attr.value,
                                )?;
                            }
                        }
                    }
                }
            }
            POLYTONE_CALLBACKS.save(
                deps.storage,
                format!(
                    "withdraw_liquidity_callback : {:?}",
                    env.block.height.to_string()
                ),
                &to_json_binary(&callback_result)?.to_string(),
            )?;

            match CONTRACT_STATE.load(deps.storage)? {
                ContractState::Distributing { coins: _ } => (),
                _ => {
                    POLYTONE_CALLBACKS.save(
                        deps.storage,
                        format!(
                            "withdraw_liquidity_callback : {:?}",
                            env.block.height.to_string()
                        ),
                        &"non-distributing-state".to_string(),
                    )?;

                    // if we are not in a distributing state, withdraw had failed.
                    // we submit the appropriate callback to the holder.
                    let holder = HOLDER_ADDRESS.load(deps.storage)?;
                    return Ok(Response::default().add_message(CosmosMsg::Wasm(
                        WasmMsg::Execute {
                            contract_addr: holder.to_string(),
                            msg: to_json_binary(&WithdrawLPMsgs::WithdrawFailed {})?,
                            funds: vec![],
                        },
                    )));
                }
            }
        }
        _ => (),
    }

    Ok(Response::default())
}

fn process_fatal_error_callback(
    env: Env,
    deps: DepsMut,
    response: String,
) -> NeutronResult<Response<NeutronMsg>> {
    POLYTONE_CALLBACKS.save(
        deps.storage,
        format!("fatal_error : {:?}", env.block.height.to_string()),
        &response,
    )?;
    Ok(Response::default())
}

fn handle_withdraw_liquidity_proxy_balances_callback(
    deps: DepsMut,
    env: Env,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
) -> NeutronResult<Response<NeutronMsg>> {
    // decode the query callback result into a vec of binaries,
    // or error out if it fails
    let response_binaries = match query_callback_result {
        Ok(val) => {
            for bin in val.clone() {
                POLYTONE_CALLBACKS.save(
                    deps.storage,
                    format!(
                        "proxy_balances_callback : {:?}",
                        env.block.height.to_string()
                    ),
                    &bin.to_base64(),
                )?;
            }
            val
        }
        Err(err) => return Err(ContractError::PolytoneError(err.error).to_neutron_std()),
    };

    // we load the lp config that was present prior
    // to attempting to exit the pool
    let mut pre_withdraw_liquidity_lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let pre_withdraw_lp_token_balance = match pre_withdraw_liquidity_lp_config
        .latest_balances
        .get(&pre_withdraw_liquidity_lp_config.lp_token_denom)
    {
        Some(bal) => bal.clone(),
        None => Coin {
            amount: Uint128::zero(),
            denom: pre_withdraw_liquidity_lp_config.lp_token_denom.to_string(),
        },
    };

    for response_binary in response_binaries {
        // parse binary into an osmosis QueryBalanceResponse
        let balance_response: QueryBalanceResponse = from_json(response_binary.clone())?;
        if let Some(balance) = balance_response.balance {
            if balance.denom == pre_withdraw_lp_token_balance.denom {
                // if proxy lp token balance did not decrease from the
                // moment when we submitted the withdraw liquidity message,
                // we assume that liquidity was withdrawn and we can
                // proceed with the distribution flow.
                if Uint128::from_str(&balance.amount)? >= pre_withdraw_lp_token_balance.amount {
                    CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;
                }
            }
            // update the latest balances map with the processed balance
            pre_withdraw_liquidity_lp_config.latest_balances.insert(
                balance.denom.to_string(),
                coin(Uint128::from_str(&balance.amount)?.u128(), balance.denom),
            );
        }
    }
    // store the latest prices in lp config
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &pre_withdraw_liquidity_lp_config)?;

    Ok(Response::default())
}

fn handle_proxy_balances_callback(
    deps: DepsMut,
    env: Env,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
) -> NeutronResult<Response<NeutronMsg>> {
    // decode the query callback result into a vec of binaries,
    // or error out if it fails
    let response_binaries = match query_callback_result {
        Ok(val) => {
            for bin in val.clone() {
                POLYTONE_CALLBACKS.save(
                    deps.storage,
                    format!(
                        "proxy_balances_callback : {:?}",
                        env.block.height.to_string()
                    ),
                    &bin.to_base64(),
                )?;
            }
            val
        }
        Err(err) => return Err(ContractError::PolytoneError(err.error).to_neutron_std()),
    };

    // store the latest prices in lp config
    LIQUIDITY_PROVISIONING_CONFIG.update(deps.storage, |mut lp_config| -> StdResult<_> {
        // process the balance responses one by one
        for response_binary in response_binaries {
            // parse binary into an osmosis QueryBalanceResponse
            let balance_response: QueryBalanceResponse = from_json(response_binary.clone())?;
            if let Some(balance) = balance_response.balance {
                // update the latest balances map with the processed balance
                lp_config.latest_balances.insert(
                    balance.denom.to_string(),
                    coin(Uint128::from_str(&balance.amount)?.u128(), balance.denom),
                );
            }
        }
        Ok(lp_config)
    })?;

    Ok(Response::default())
}

pub fn get_note_execute_neutron_msg(
    msgs: Vec<CosmosMsg>,
    ibc_timeout: Uint64,
    note_address: Addr,
    callback: Option<CallbackRequest>,
) -> NeutronResult<CosmosMsg<NeutronMsg>> {
    let polytone_msg = get_polytone_execute_msg_binary(msgs, callback, ibc_timeout)?;

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_msg,
        funds: vec![],
    }))
}

pub fn get_ibc_withdraw_coin_message(
    channel_id: String,
    to_address: String,
    amount: Coin,
    timeout: IbcTimeout,
) -> CosmosMsg {
    let msg = IbcMsg::Transfer {
        channel_id,
        to_address,
        amount,
        timeout,
    };

    msg.into()
}

pub fn get_ibc_pfm_withdraw_coin_message(
    channel_id: String,
    from_address: String,
    to_address: String,
    amount: Coin,
    timeout_timestamp_nanos: u64,
    memo: String,
) -> CosmosMsg {
    use prost::Message;

    let ibc_message = osmosis_std::types::ibc::applications::transfer::v1::MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: channel_id,
        token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: amount.denom,
            amount: amount.amount.to_string(),
        }),
        sender: from_address,
        receiver: to_address,
        timeout_height: None,
        timeout_timestamp: timeout_timestamp_nanos,
        memo,
    };

    cosmwasm_std::CosmosMsg::Stargate {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary(ibc_message.encode_to_vec()),
    }
}

pub fn get_proxy_query_balances_message(
    env: Env,
    proxy_address: String,
    note_address: String,
    lp_config: LiquidityProvisionConfig,
    ibc_config: IbcConfig,
    callback_id: u8,
) -> StdResult<WasmMsg> {
    let proxy_coin_1_balance_request: QueryRequest<Empty> =
        osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest {
            address: proxy_address.to_string(),
            denom: lp_config.party_1_denom_info.osmosis_coin.denom,
        }
        .into();
    let proxy_coin_2_balance_request: QueryRequest<Empty> =
        osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest {
            address: proxy_address.to_string(),
            denom: lp_config.party_2_denom_info.osmosis_coin.denom,
        }
        .into();
    let proxy_gamm_balance_request: QueryRequest<Empty> =
        osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest {
            address: proxy_address,
            denom: lp_config.lp_token_denom,
        }
        .into();

    let polytone_query_msg_binary = get_polytone_query_msg_binary(
        vec![
            proxy_coin_1_balance_request,
            proxy_coin_2_balance_request,
            proxy_gamm_balance_request,
        ],
        CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&callback_id)?,
        },
        ibc_config.osmo_ibc_timeout,
    )?;

    Ok(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_query_msg_binary,
        funds: vec![],
    })
}
