use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg, WasmMsg, Uint64, QueryRequest, Empty, from_json, StdError,
};
use covenant_utils::{get_polytone_execute_msg_binary, query_polytone_proxy_address, get_polytone_query_msg_binary};
use cw2::set_contract_version;
use osmosis_std::types::{
    osmosis::gamm::v1beta1::{MsgJoinPool, Pool},
    cosmos::base::v1beta1::Coin as OsmosisCoin,
};

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, ProvidedLiquidityInfo, QueryMsg},
    state::{HOLDER_ADDRESS, PROVIDED_LIQUIDITY_INFO, NOTE_ADDRESS, PROXY_ADDRESS, COIN_1, COIN_2, CALLBACKS, LATEST_OSMO_POOL_RESPONSE},
};

use polytone::callbacks::{Callback as PolytoneCallback, CallbackMessage, ErrorResponse, ExecutionResponse, CallbackRequest};

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate the contract addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let _pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;
    let note_addr = deps.api.addr_validate(&msg.note_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant module addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;

    NOTE_ADDRESS.save(deps.storage, &note_addr)?;
    COIN_1.save(deps.storage, &msg.coin_1)?;
    COIN_2.save(deps.storage, &msg.coin_2)?;
    CALLBACKS.save(deps.storage, &Vec::new())?;
    LATEST_OSMO_POOL_RESPONSE.save(deps.storage, &Binary::default())?;

    // we begin with no liquidity provided
    PROVIDED_LIQUIDITY_INFO.save(
        deps.storage,
        &ProvidedLiquidityInfo {
            provided_amount_a: Uint128::zero(),
            provided_amount_b: Uint128::zero(),
        },
    )?;

    Ok(Response::default()
        // .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "lp_instantiate")
        .add_attribute("clock_addr", clock_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Callback(callback_msg) => try_handle_callback(deps, info, callback_msg),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_handle_callback(deps: DepsMut, info: MessageInfo, msg: CallbackMessage) -> Result<Response, ContractError> {
    // only the note can submit a callback
    if info.sender != NOTE_ADDRESS.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    match msg.result {
        PolytoneCallback::Query(resp) => process_query_callback(deps,resp),
        PolytoneCallback::Execute(resp) => process_execute_callback(deps,resp),
        PolytoneCallback::FatalError(resp) => process_fatal_error_callback(deps, resp),
    }
}

fn process_query_callback(
    deps: DepsMut,
    query_callback_result: Result<Vec<Binary>, ErrorResponse>,
) -> Result<Response, ContractError> {
    let entries = match query_callback_result {
        Ok(response) => {
            if let Some(bin) = response.get(0) {
                LATEST_OSMO_POOL_RESPONSE.save(deps.storage, &bin)?;
                CONTRACT_STATE.save(deps.storage, &ContractState::ProxyFunded)?;
            };

            response.into_iter().map(|resp| resp.to_string()).collect()
        },
        Err(err) => vec![format!("{:?} : {:?}", err.message_index, err.error)],
    };

    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

fn process_execute_callback(
    deps: DepsMut,
    execute_callback_result: Result<ExecutionResponse, String>,
) -> Result<Response, ContractError> {
    let entries = match execute_callback_result {
        Ok(execution_response) => execution_response.result
            .into_iter()
            .map(|r| {
                match r.data {
                    Some(data) => data.to_string(),
                    None => "none".to_string(),
                }
            })
            .collect(),
        Err(err) => vec![err],
    };


    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.extend(entries);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

fn process_fatal_error_callback(
    deps: DepsMut,
    response: String,
) -> Result<Response, ContractError> {
    let mut callbacks = CALLBACKS.load(deps.storage)?;
    callbacks.push(response);
    CALLBACKS.save(deps.storage, &callbacks)?;

    Ok(Response::default())
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    // verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_query_proxy_address(deps, env),
        ContractState::ProxyCreated => try_query_pool(deps, env),
        ContractState::ProxyFunded => try_lp(deps, env),
        ContractState::Active => todo!(),
        ContractState::Complete => todo!(),
    }
}

fn try_query_pool(deps: DepsMut, env: Env) -> Result<Response, ContractError> {

    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let query_pool_request: QueryRequest<Empty> = osmosis_std::types::osmosis::gamm::v1beta1::QueryPoolRequest {
        pool_id: 1,
    }
    .into();


    let polytone_query_msg_binary = get_polytone_query_msg_binary(
        vec![query_pool_request],
        CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&"osmosis_std::types::osmosis::gamm::v1beta1::QueryPoolRequest")?,
        },
        Uint64::new(200),
    )?;

    let note_msg = CosmosMsg::Wasm(
        WasmMsg::Execute {
            contract_addr: note_address.to_string(),
            msg: polytone_query_msg_binary,
            funds: vec![],
        }
    );

    Ok(Response::default()
        .add_message(note_msg)
        .add_attribute("method", "try_query_pool"))
}


fn try_create_proxy(deps: DepsMut, env: Env, note_address: String) -> Result<Response, ContractError> {
    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary("proxy_created")?,
        }),
        Uint64::new(200),
    )?;

    let note_msg = CosmosMsg::Wasm(
        WasmMsg::Execute {
            contract_addr: note_address.to_string(),
            msg: polytone_execute_msg_binary,
            funds: vec![],
        }
    );
    Ok(Response::default()
        .add_message(note_msg)
        .add_attribute("method", "try_create_proxy"))
}

fn try_query_proxy_address(deps: DepsMut, env: Env) -> Result<Response, ContractError> {

    let note_address = NOTE_ADDRESS.load(deps.storage)?;

    let proxy_address = query_polytone_proxy_address(
        env.contract.address.to_string(),
        note_address.to_string(),
        deps.querier,
    )?;

    match proxy_address {
        // if proxy is created, we save it and advance the state machine
        Some(addr) => {
            PROXY_ADDRESS.save(deps.storage, &addr)?;
            CONTRACT_STATE.save(deps.storage, &ContractState::ProxyCreated)?;

            Ok(Response::default()
                .add_attribute("method", "try_query_proxy_address")
                .add_attribute("proxy_address", addr))
        },
        // if proxy is not created, try to create it
        None => try_create_proxy(deps, env, note_address.to_string()),
    }
}

fn try_fund_proxy(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    Ok(Response::default().add_attribute("method", "todo"))
}

fn try_lp(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // this call means proxy is created, funded, and we are ready to LP
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    let coin_1 = COIN_1.load(deps.storage)?;
    let coin_2 = COIN_2.load(deps.storage)?;

    let latest_osmo_pool_response = LATEST_OSMO_POOL_RESPONSE.load(deps.storage)?;
    let pool = from_json::<osmosis_std::types::osmosis::gamm::v1beta1::Pool>(latest_osmo_pool_response)?;

    let token_in_maxs = vec![
        OsmosisCoin::from(coin_1.clone()),
        OsmosisCoin::from(coin_2.clone()),
    ];

    let total_shares = match pool.total_shares {
        Some(gamm_shares) => Uint128::from_str(&gamm_shares.amount)?,
        None => return Err(ContractError::Std(StdError::generic_err("not good"))),
    };

    let pool_assets: Vec<OsmosisCoin> = pool.pool_assets.into_iter()
        .filter_map(|asset| asset.token)
        .collect();

    let (pool_asset_1_amount, pool_asset_2_amount) = match (pool_assets.get(0), pool_assets.get(1)) {
        (Some(pool_asset_1), Some(pool_asset_2)) => {
            if pool_asset_1.denom == coin_1.denom && pool_asset_2.denom == coin_2.denom {
                (pool_asset_1.amount.to_string(), pool_asset_2.amount.to_string())
            } else {
                (pool_asset_2.amount.to_string(), pool_asset_1.amount.to_string())
            }
        },
        _ => return Err(ContractError::Std(StdError::generic_err("not good"))),
    };

    let share_out_amount = std::cmp::min(
        coin_1.amount.multiply_ratio(
            total_shares,
            Uint128::from_str(&pool_asset_1_amount)?.u128(),
        ),
        coin_2.amount.multiply_ratio(
            total_shares,
            Uint128::from_str(&pool_asset_2_amount)?.u128(),
        ),
    );
    let tokens_string = format!("{:?} + {:?}", coin_1.to_string(), coin_2.to_string());

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: proxy_address,
        pool_id: 1,
        // exact number of shares we wish to receive
        share_out_amount: share_out_amount.to_string(),
        token_in_maxs,
    }
    .into();

    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![osmo_msg],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&tokens_string)?,
        }),
        Uint64::new(200),
    )?;

    let note_msg = CosmosMsg::Wasm(
        WasmMsg::Execute {
            contract_addr: note_address.to_string(),
            msg: polytone_execute_msg_binary,
            funds: vec![],
        }
    );

    Ok(Response::default()
        .add_message(note_msg)
        .add_attribute("method", "try_lp"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ProvidedLiquidityInfo {} => Ok(to_json_binary(
            &PROVIDED_LIQUIDITY_INFO.load(deps.storage)?,
        )?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
        QueryMsg::ProxyAddress {} => Ok(to_json_binary(&PROXY_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::Callbacks {} => Ok(to_json_binary(&CALLBACKS.load(deps.storage)?)?),
        QueryMsg::LatestPoolState {} => {
            let latest_pool_binary = LATEST_OSMO_POOL_RESPONSE.load(deps.storage)?;
            Ok(latest_pool_binary)
        }
    }
}
