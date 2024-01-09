use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, QueryRequest, Response, StdResult, Uint128, Uint64, WasmMsg,
};
use covenant_utils::{
    default_ibc_fee, get_polytone_execute_msg_binary, get_polytone_query_msg_binary,
    query_polytone_proxy_address,
};
use cw2::set_contract_version;
use neutron_sdk::{
    bindings::msg::NeutronMsg, sudo::msg::RequestPacketTimeoutHeight, NeutronResult,
};
use polytone::callbacks::CallbackRequest;

use crate::{
    error::ContractError,
    msg::{
        ContractState, ExecuteMsg, ForwardMetadata, InstantiateMsg,
        PacketMetadata, PartyChainInfo, QueryMsg, LiquidityProvisionConfig, IbcConfig,
    },
    state::{
        HOLDER_ADDRESS, NOTE_ADDRESS,
        PROXY_ADDRESS, LIQUIDITY_PROVISIONING_CONFIG, IBC_CONFIG,
    }, polytone_handlers::try_handle_callback,
};


use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const LIQUIDITY_PROVIDED_CALLBACK_ID: u8 = 1;
pub(crate) const PROXY_BALANCES_CALLBACK_ID: u8 = 2;
pub(crate) const PROXY_CREATED_CALLBACK_ID: u8 = 3;

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
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;
    let note_addr = deps.api.addr_validate(&msg.note_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant contract addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;
    NOTE_ADDRESS.save(deps.storage, &note_addr)?;

    // initialize polytone state sync related items
    let latest_balances: HashMap<String, Coin> = HashMap::new();
    let denom_config = LiquidityProvisionConfig {
        latest_balances,
        party_1_denom_info: msg.party_1_denom_info,
        party_2_denom_info: msg.party_2_denom_info,
        pool_id: msg.pool_id,
        outpost: msg.osmo_outpost,
        lp_token_denom: msg.lp_token_denom,
    };
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &denom_config)?;

    let ibc_config = IbcConfig {
        party_1_chain_info: msg.party_1_chain_info,
        party_2_chain_info: msg.party_2_chain_info,
        osmo_to_neutron_channel_id: msg.osmo_to_neutron_channel_id,
        osmo_ibc_timeout: msg.osmo_ibc_timeout,
    };
    IBC_CONFIG.save(deps.storage, &ibc_config)?;

    Ok(Response::default()
        // .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "osmosis_lp_instantiate")
        .add_attribute("clock_addr", clock_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Callback(callback_msg) => try_handle_callback(env, deps, info, callback_msg),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, _info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    // verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        // create a proxy account
        ContractState::Instantiated => try_query_proxy_address(deps, env),
        // fund the proxy account
        ContractState::ProxyCreated => {
            let denom_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

            // if coin balance is stored, get it.
            // otherwise return default which we assert below.
            let proxy_party_1_coin = match denom_config.get_party_1_denom_balance() {
                Some(c) => c.clone(),
                None => Coin::default(),
            };
            let proxy_party_2_coin = match denom_config.get_party_2_denom_balance() {
                Some(c) => c.clone(),
                None => Coin::default(),
            };

            // if no balances are stored, we query them
            if proxy_party_1_coin == Coin::default() && proxy_party_2_coin == Coin::default() {
                query_proxy_balances(deps, env)
            } else {
                // check if proxy contains balances that both parties should deliver
                let coin_1_funded = proxy_party_1_coin.amount
                    >= denom_config.party_1_denom_info.osmosis_coin.amount;
                let coin_2_funded = proxy_party_2_coin.amount
                    >= denom_config.party_2_denom_info.osmosis_coin.amount;
                // if either coin is not funded, we attempt to do so
                if !coin_1_funded || !coin_2_funded {
                    try_fund_proxy(deps, env)
                } else {
                    // otherwise we advance the state machine
                    CONTRACT_STATE.save(deps.storage, &ContractState::ProxyFunded)?;
                    Ok(Response::default()
                        .add_attribute("method", "try_tick")
                        .add_attribute("contract_state", "proxy_funded"))
                }
            }
        }
        // attempt to provide liquidity
        ContractState::ProxyFunded => try_lp_outpost(deps, env),
        // no longer accept any actions
        ContractState::Complete => {
            Err(ContractError::StateMachineError("complete".to_string()).to_neutron_std())
        }
    }
}

fn try_lp_outpost(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    // this call means proxy is created, funded, and we are ready to LP
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    // let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

    let mut funds_to_send = vec![];
    if let Some(c) = lp_config.get_party_1_denom_balance() {
        funds_to_send.push(c.clone());
    }
    if let Some(c) = lp_config.get_party_2_denom_balance() {
        funds_to_send.push(c.clone());
    }

    let outpost_provide_liquidity_msg =
        covenant_outpost_osmo_liquid_pooler::msg::ExecuteMsg::ProvideLiquidity {
            pool_id: Uint64::new(lp_config.pool_id.u64()),
            min_pool_asset_ratio: Decimal::zero(),
            max_pool_asset_ratio: Decimal::one(),
            slippage_tolerance: Decimal::from_ratio(Uint128::new(5), Uint128::new(100)),
        };

    let outpost_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: lp_config.outpost.to_string(),
        msg: to_json_binary(&outpost_provide_liquidity_msg)?,
        funds: funds_to_send, // entire proxy balances
    }
    .into();


    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        // include gamm token transfer msg back to holder after osmo_msg
        vec![outpost_msg],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&LIQUIDITY_PROVIDED_CALLBACK_ID)?,
        }),
        ibc_config.osmo_ibc_timeout,
    )?;

    let note_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_execute_msg_binary,
        funds: vec![],
    });

    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    let note_query_balances_message = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config,
        ibc_config,
    )?;

    Ok(Response::default()
        .add_message(note_msg)
        .add_message(note_query_balances_message)
        .add_attribute("method", "try_lp"))
}

fn get_proxy_query_balances_message(
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
        vec![proxy_coin_1_balance_request, proxy_coin_2_balance_request, proxy_gamm_balance_request],
        CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&PROXY_BALANCES_CALLBACK_ID)?,
        },
        ibc_config.osmo_ibc_timeout,
    )?;

    Ok(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_query_msg_binary,
        funds: vec![],
    })
}

fn query_proxy_balances(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

    let note_balance_query_msg = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config,
        ibc_config,
    )?;
    Ok(Response::default()
        .add_message(note_balance_query_msg)
        .add_attribute("method", "try_query_proxy_balances"))
}

fn try_create_proxy(
    deps: DepsMut,
    env: Env,
    note_address: String,
) -> NeutronResult<Response<NeutronMsg>> {
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&PROXY_CREATED_CALLBACK_ID)?,
        }),
        ibc_config.osmo_ibc_timeout,
    )?;

    let note_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_execute_msg_binary,
        funds: vec![],
    });
    Ok(Response::default()
        .add_message(note_msg)
        .add_attribute("method", "try_create_proxy"))
}

// TODO: remake this into something like `try_create_proxy`
fn try_query_proxy_address(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
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
        }
        // if proxy is not created, try to create it
        None => try_create_proxy(deps, env, note_address.to_string()),
    }
}

fn try_fund_proxy(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut denom_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    let coin_1_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        denom_config.party_1_denom_info.neutron_denom.to_string(),
    )?;

    let coin_2_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        denom_config.party_2_denom_info.neutron_denom.to_string(),
    )?;

    // if either available balance is not sufficient,
    // we reset the latest proxy balance to `None`.
    // this will trigger a query on following tick.
    if denom_config.party_1_denom_info.osmosis_coin.amount > coin_1_bal.amount
        || denom_config.party_2_denom_info.osmosis_coin.amount > coin_2_bal.amount
    {
        // remove party denom entries from the balances map.
        // this will trigger a proxy balance query on the following tick.
        denom_config.latest_balances.remove(
            &denom_config
                .party_1_denom_info
                .osmosis_coin
                .denom
                .to_string(),
        );
        denom_config.latest_balances.remove(
            &denom_config
                .party_2_denom_info
                .osmosis_coin
                .denom
                .to_string(),
        );
        LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &denom_config)?;
        return Ok(Response::default()
            .add_attribute("method", "try_fund_proxy")
            .add_attribute("result", "insufficient_balances"));
    }

    let mut transfer_messages = vec![];

    if coin_1_bal.amount > Uint128::zero() {
        transfer_messages.push(get_ibc_transfer_message(
            ibc_config.party_1_chain_info,
            env.clone(),
            coin_1_bal,
            proxy_address.to_string(),
        )?);
    }
    if coin_2_bal.amount > Uint128::zero() {
        transfer_messages.push(get_ibc_transfer_message(
            ibc_config.party_2_chain_info,
            env,
            coin_2_bal,
            proxy_address,
        )?);
    }

    Ok(Response::default()
        .add_messages(transfer_messages)
        .add_attribute("method", "try_fund_proxy"))
}

fn get_ibc_transfer_message(
    party_chain_info: PartyChainInfo,
    env: Env,
    coin: Coin,
    proxy_address: String,
) -> StdResult<NeutronMsg> {
    // depending on whether pfm is configured,
    // we return a ibc transfer message
    match party_chain_info.pfm {
        // pfm necesary, we configure the memo
        Some(forward_metadata) => Ok(NeutronMsg::IbcTransfer {
            source_port: party_chain_info.neutron_to_party_chain_port,
            source_channel: party_chain_info.neutron_to_party_chain_channel,
            token: coin,
            sender: env.contract.address.to_string(),
            receiver: forward_metadata.receiver,
            timeout_height: RequestPacketTimeoutHeight {
                revision_number: None,
                revision_height: None,
            },
            timeout_timestamp: env
                .block
                .time
                .plus_seconds(party_chain_info.ibc_timeout.u64())
                .nanos(),
            memo: to_json_string(&PacketMetadata {
                forward: Some(ForwardMetadata {
                    receiver: proxy_address.to_string(),
                    port: forward_metadata.port,
                    channel: forward_metadata.channel,
                }),
            })?,
            fee: default_ibc_fee(),
        }),
        // no pfm necessary, we do a regular transfer
        None => Ok(NeutronMsg::IbcTransfer {
            source_port: party_chain_info.neutron_to_party_chain_port,
            source_channel: party_chain_info.neutron_to_party_chain_channel,
            token: coin,
            sender: env.contract.address.to_string(),
            receiver: proxy_address.to_string(),
            timeout_height: RequestPacketTimeoutHeight {
                revision_number: None,
                revision_height: None,
            },
            timeout_timestamp: env
                .block
                .time
                .plus_seconds(party_chain_info.ibc_timeout.u64())
                .nanos(),
            memo: "".to_string(),
            fee: default_ibc_fee(),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
        QueryMsg::ProxyAddress {} => Ok(to_json_binary(&PROXY_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ProxyBalances {} => {
            let config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
            let vec: Vec<Coin> = config.latest_balances.iter()
                .map(|(_, coin)| coin.to_owned())
                .collect();
            Ok(to_json_binary(&vec)?)
        }
    }
}
