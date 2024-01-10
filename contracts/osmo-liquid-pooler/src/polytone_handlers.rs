use std::str::FromStr;

use cosmwasm_std::{
    from_json, Binary, Coin, CosmosMsg, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response,
    Uint128, WasmMsg, StdResult, Addr, Uint64, coin,
};
use covenant_utils::{get_polytone_execute_msg_binary, query_polytone_proxy_address};
use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

use crate::{
    contract::{PROXY_BALANCES_QUERY_CALLBACK_ID, PROVIDE_LIQUIDITY_CALLBACK_ID, CREATE_PROXY_CALLBACK_ID

    },
    error::ContractError,
    msg::ContractState,
    state::{CONTRACT_STATE, IBC_CONFIG, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS, HOLDER_ADDRESS, POLYTONE_CALLBACKS, PROXY_ADDRESS},
};

use polytone::callbacks::{
    Callback as PolytoneCallback, CallbackMessage, ErrorResponse, ExecutionResponse, CallbackRequest,
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
        PolytoneCallback::Execute(resp) => process_execute_callback(env, deps, resp, msg.initiator_msg),
        PolytoneCallback::FatalError(resp) => process_fatal_error_callback(deps, resp),
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
        PROXY_BALANCES_QUERY_CALLBACK_ID => handle_proxy_balances_callback(deps, env, query_callback_result),
        _ => {
            return Err(ContractError::PolytoneError(format!(
                "unexpected callback id: {:?}",
                initiator_msg
            ))
            .to_neutron_std())
        }
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
            for submsg_response in callback_result.result {
                if submsg_response.data.is_some() {
                    if let Some(response_binary) = submsg_response.data {
                        POLYTONE_CALLBACKS.save(deps.storage,
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

            if let Some(addr) = proxy_address {
                PROXY_ADDRESS.save(deps.storage, &addr)?;
                CONTRACT_STATE.save(deps.storage, &ContractState::ProxyCreated)?;
            }
        }
        _ => (),
    }

    Ok(Response::default())
}

fn process_fatal_error_callback(
    _deps: DepsMut,
    _response: String,
) -> NeutronResult<Response<NeutronMsg>> {
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

    // store the latest prices in lp config
    let lp_config = LIQUIDITY_PROVISIONING_CONFIG.update(deps.storage, |mut lp_config| -> StdResult<_> {
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
    let polytone_msg = get_polytone_execute_msg_binary(
        msgs,
        callback,
        ibc_timeout,
    )?;

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
