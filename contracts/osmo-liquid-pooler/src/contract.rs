use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg, WasmMsg, Uint64,
};
use covenant_utils::{get_polytone_execute_msg_binary, query_polytone_proxy_address};
use cw2::set_contract_version;
use osmosis_std::types::{osmosis::gamm::v1beta1::MsgJoinPool, cosmos};
use polytone::callbacks::CallbackRequest;

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, ProvidedLiquidityInfo, QueryMsg},
    state::{HOLDER_ADDRESS, PROVIDED_LIQUIDITY_INFO, NOTE_ADDRESS, PROXY_ADDRESS, COIN_1, COIN_2},
};

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
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    // verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_query_proxy_address(deps, env),
        ContractState::ProxyCreated => try_lp(deps, env),
        ContractState::ProxyFunded => try_lp(deps, env),
        ContractState::Active => todo!(),
        ContractState::Complete => todo!(),
    }
}

fn try_create_proxy(deps: DepsMut, _env: Env, note_address: String) -> Result<Response, ContractError> {
    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        None,
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
    Ok(Response::default().add_attribute("method", "try_lp"))
}

fn try_lp(deps: DepsMut, env: Env) -> Result<Response, ContractError> {

    // this call means proxy is created, funded, and we are ready to LP
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    let coin_1 = COIN_1.load(deps.storage)?;
    let coin_2 = COIN_2.load(deps.storage)?;

    // hardcoded values for testing
    let total_share = Uint128::new(100000000000000000000);
    let pool_asset_1 = cosmos::base::v1beta1::Coin {
        denom: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9".to_string(),
        amount: "5000000000000".to_string(),
    };
    let pool_asset_2 = cosmos::base::v1beta1::Coin {
        denom: "uosmo".to_string(),
        amount: "55000000000000".to_string(),
    };

    let share_out_amount = std::cmp::min(
        coin_1.amount.multiply_ratio(
            total_share,
            Uint128::from_str(&pool_asset_1.amount)?.u128(),
        ),
        coin_2.amount.multiply_ratio(
            total_share,
            Uint128::from_str(&pool_asset_2.amount)?.u128(),
        ),
    );

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: proxy_address,
        pool_id: 1,
        // exact number of shares we wish to receive
        share_out_amount: share_out_amount.to_string(),
        token_in_maxs: vec![
            cosmos::base::v1beta1::Coin::from(coin_1),
            cosmos::base::v1beta1::Coin::from(coin_2),
        ],
    }
    .into();

    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![osmo_msg],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary("nice")?,
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
    }
}
