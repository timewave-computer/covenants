use std::str::FromStr;

use cosmwasm_std::{
    coin, from_json, to_json_binary, Addr, Binary, Coin, CosmosMsg, DepsMut, Empty, Env, IbcMsg,
    IbcTimeout, MessageInfo, QueryRequest, Response, StdResult, Uint128, Uint64, WasmMsg,
};
use covenant_utils::polytone::{
    get_polytone_execute_msg_binary, get_polytone_query_msg_binary, query_polytone_proxy_address,
};
use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

use crate::{
    contract::{
        CREATE_PROXY_CALLBACK_ID, PROVIDE_LIQUIDITY_CALLBACK_ID, PROXY_BALANCES_QUERY_CALLBACK_ID,
    },
    error::ContractError,
    msg::{ContractState, IbcConfig, LiquidityProvisionConfig},
    state::{
        CONTRACT_STATE, IBC_CONFIG, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS,
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
    let callback_result = match execute_callback_result {
        Ok(val) => val,
        Err(e) => return Err(ContractError::PolytoneError(e).to_neutron_std()),
    };

    match initiator_msg {
        PROVIDE_LIQUIDITY_CALLBACK_ID => {
            POLYTONE_CALLBACKS.save(
                deps.storage,
                format!(
                    "provide_liquidity_callback : {:?}",
                    env.block.time.to_string()
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
                format!("create_proxy_callback : {:?}", env.block.time.to_string()),
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
        format!("fatal_error : {:?}", env.block.time.to_string()),
        &response,
    )?;
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
        Ok(val) => val,
        Err(err) => return Err(ContractError::PolytoneError(err.error).to_neutron_std()),
    };

    for bin in response_binaries.clone() {
        POLYTONE_CALLBACKS.save(
            deps.storage,
            format!("proxy_balances_callback : {:?}", env.block.time.to_string()),
            &bin.to_base64(),
        )?;
    }

    // store the latest prices in lp config
    let lp_config =
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

    // if latest balances contain any gamm tokens,
    // we withdraw them to this contract
    if let Some(coin) = lp_config.latest_balances.get(&lp_config.lp_token_denom) {
        if !coin.amount.is_zero() {
            let ibc_config = IBC_CONFIG.load(deps.storage)?;
            let note_address = NOTE_ADDRESS.load(deps.storage)?;

            let withdraw_message = get_withdraw_lp_tokens_message(
                ibc_config.osmo_to_neutron_channel_id,
                env.contract.address.to_string(),
                coin.clone(),
                IbcTimeout::with_timestamp(
                    env.block
                        .time
                        .plus_seconds(ibc_config.osmo_ibc_timeout.u64()),
                ),
            );
            let note_msg = get_note_execute_neutron_msg(
                vec![withdraw_message],
                ibc_config.osmo_ibc_timeout,
                note_address,
                None,
            )?;

            return Ok(Response::default().add_message(note_msg));
        }
    }

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

fn get_withdraw_lp_tokens_message(
    channel_id: String,
    to_address: String,
    amount: Coin,
    timeout: IbcTimeout,
) -> CosmosMsg {
    let withdraw_lp_msg = IbcMsg::Transfer {
        channel_id,
        to_address,
        amount,
        timeout,
    };
    CosmosMsg::Ibc(withdraw_lp_msg)
}

pub fn get_proxy_query_balances_message(
    env: Env,
    proxy_address: String,
    note_address: String,
    lp_config: LiquidityProvisionConfig,
    ibc_config: IbcConfig,
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
            msg: to_json_binary(&PROXY_BALANCES_QUERY_CALLBACK_ID)?,
        },
        ibc_config.osmo_ibc_timeout,
    )?;

    Ok(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_query_msg_binary,
        funds: vec![],
    })
}
