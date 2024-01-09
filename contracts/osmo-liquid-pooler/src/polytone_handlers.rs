use std::str::FromStr;

use cosmwasm_std::{from_json, Binary, Coin, DepsMut, Response, Uint128, MessageInfo, CosmosMsg, IbcMsg, IbcTimeout, Env, WasmMsg};
use covenant_utils::get_polytone_execute_msg_binary;
use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

use crate::{
    msg::ContractState,
    state::{CONTRACT_STATE, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS, IBC_CONFIG},
    contract::{PROXY_BALANCES_CALLBACK_ID, LIQUIDITY_PROVIDED_CALLBACK_ID, PROXY_CREATED_CALLBACK_ID}, error::ContractError,
};

use polytone::callbacks::{Callback as PolytoneCallback, CallbackMessage, ErrorResponse, ExecutionResponse};


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
        PolytoneCallback::Query(resp) =>
            process_query_callback(env, deps, resp, msg.initiator_msg),
        PolytoneCallback::Execute(resp) =>
            process_execute_callback(deps, resp, msg.initiator_msg),
        PolytoneCallback::FatalError(resp) =>
            process_fatal_error_callback(deps, resp),
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
        PROXY_BALANCES_CALLBACK_ID => {
            // decode the query callback result into a vec of binaries,
            // or error out if it fails
            let response_binaries = match query_callback_result {
                Ok(val) => val,
                Err(err) => return Err(ContractError::PolytoneError(err.error).to_neutron_std()),
            };

            let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

            // process the balance responses one by one
            for response_binary in response_binaries {
                // parse binary into an osmosis QueryBalanceResponse
                let balance_response: QueryBalanceResponse = from_json(response_binary.clone())?;
                if let Some(balance) = balance_response.balance {
                    // update the latest balances map with the processed balance
                    lp_config.latest_balances.insert(
                        balance.denom.to_string(),
                        Coin { denom: balance.denom, amount: Uint128::from_str(&balance.amount)? },
                    );
                }
            }

            LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

            // if latest balances contain gamm token
            if let Some(coin) = lp_config.latest_balances.get(&lp_config.lp_token_denom) {
                if !coin.amount.is_zero() {
                    let ibc_config = IBC_CONFIG.load(deps.storage)?;
                    let withdraw_message = get_withdraw_lp_tokens_message(
                        ibc_config.osmo_to_neutron_channel_id,
                        env.contract.address.to_string(),
                        coin.clone(),
                        IbcTimeout::with_timestamp(
                            env.block.time.plus_seconds(ibc_config.osmo_ibc_timeout.u64()),
                        ),
                    );
                    let polytone_msg = get_polytone_execute_msg_binary(
                        vec![withdraw_message],
                        None,
                        ibc_config.osmo_ibc_timeout,
                    )?;
                    let note_address = NOTE_ADDRESS.load(deps.storage)?;
                    let note_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: note_address.to_string(),
                        msg: polytone_msg,
                        funds: vec![],
                    });

                    return Ok(Response::default().add_message(note_msg));
                }
            }
            // TODO: if the latest proxy balances contain a non-zero lp token balance,
            // fire an ibc-withdraw message to the proxy
        },
        _ => return Err(ContractError::PolytoneError(
            format!("unexpected callback id: {:?}", initiator_msg),
        ).to_neutron_std()),
    }

    Ok(Response::default())
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

fn process_execute_callback(
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
        LIQUIDITY_PROVIDED_CALLBACK_ID => {
            for submsg_response in callback_result.result {
                if let Some(_) = submsg_response.data {
                    // todo: do we want to do something here?
                }
            }
        },
        PROXY_CREATED_CALLBACK_ID => {
            for submsg_response in callback_result.result {
                if let Some(_) = submsg_response.data {
                    CONTRACT_STATE.save(deps.storage, &ContractState::ProxyCreated)?;
                }
            }
        },
        _ => ()
    }

    Ok(Response::default())
}

fn process_fatal_error_callback(
    deps: DepsMut,
    response: String,
) -> NeutronResult<Response<NeutronMsg>> {

    Ok(Response::default())
}
