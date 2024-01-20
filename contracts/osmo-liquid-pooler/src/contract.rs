use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg, Decimal,
};
use covenant_utils::{default_ibc_fee, get_polytone_execute_msg_binary};
use cw2::set_contract_version;
use neutron_sdk::{
    bindings::msg::NeutronMsg, sudo::msg::RequestPacketTimeoutHeight, NeutronResult,
};
use polytone::callbacks::CallbackRequest;

use crate::{
    error::ContractError,
    msg::{
        ContractState, ExecuteMsg, ForwardMetadata, IbcConfig, InstantiateMsg,
        LiquidityProvisionConfig, MigrateMsg, PacketMetadata, PartyChainInfo, QueryMsg,
    },
    polytone_handlers::{
        get_note_execute_neutron_msg, get_proxy_query_balances_message, try_handle_callback,
    },
    state::{
        HOLDER_ADDRESS, IBC_CONFIG, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS,
        POLYTONE_CALLBACKS, PROXY_ADDRESS,
    },
};

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const PROVIDE_LIQUIDITY_CALLBACK_ID: u8 = 1;
pub(crate) const PROXY_BALANCES_QUERY_CALLBACK_ID: u8 = 2;
pub(crate) const CREATE_PROXY_CALLBACK_ID: u8 = 3;

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
    let lp_config = LiquidityProvisionConfig {
        latest_balances,
        party_1_denom_info: msg.party_1_denom_info,
        party_2_denom_info: msg.party_2_denom_info,
        pool_id: msg.pool_id,
        outpost: msg.osmo_outpost,
        lp_token_denom: msg.lp_token_denom,
        slippage_tolerance: msg.slippage_tolerance,
        expected_spot_price: msg.expected_spot_price,
        acceptable_price_spread: msg.acceptable_price_spread,
    };
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

    let ibc_config = IbcConfig {
        party_1_chain_info: msg.party_1_chain_info,
        party_2_chain_info: msg.party_2_chain_info,
        osmo_to_neutron_channel_id: msg.osmo_to_neutron_channel_id,
        osmo_ibc_timeout: msg.osmo_ibc_timeout,
    };
    IBC_CONFIG.save(deps.storage, &ibc_config)?;

    Ok(Response::default()
        // TODO: reenable when integrating holder
        // .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "osmosis_lp_instantiate")
        .add_attribute("contract_state", "instantiated")
        .add_attributes(lp_config.to_response_attributes())
        .add_attributes(ibc_config.to_response_attributes())
        .add_attribute("note_address", note_addr)
        .add_attribute("holder_address", holder_addr)
        .add_attribute("clock_address", clock_addr))
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
        ExecuteMsg::Withdraw { percentage } => try_withdraw(deps, env, info, percentage),
    }
}


fn try_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    percent: Option<Decimal>,
) -> NeutronResult<Response<NeutronMsg>> {
    // TODO
    Ok(Response::default())
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, _info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    // verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    match CONTRACT_STATE.load(deps.storage)? {
        // create a proxy account
        ContractState::Instantiated => try_create_proxy(deps, env),
        // fund the proxy account
        ContractState::ProxyCreated => try_deliver_funds(deps, env),
        // attempt to provide liquidity
        ContractState::ProxyFunded => try_provide_liquidity(deps, env),
        // no longer accept any actions
        ContractState::Complete => {
            Err(ContractError::StateMachineError("complete".to_string()).to_neutron_std())
        }
    }
}

/// fires an empty message to the note contract. this in turn triggers
/// the voice contract to create a proxy for this contract.
/// state is advanced from `instantiated` to `proxy_created` on the
/// polytone callback, where we query the note for remote address.
/// if address is found, we store it in PROXY_ADDRESS and advance the
/// state to `proxy_created`.
/// see polytone_handlers `process_execute_callback` match statement
/// handling the CREATE_PROXY_CALLBACK_ID for details.
fn try_create_proxy(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&CREATE_PROXY_CALLBACK_ID)?,
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

fn try_deliver_funds(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

    // check if both balances have a recent query
    match (
        lp_config.get_party_1_proxy_balance(),
        lp_config.get_party_2_proxy_balance(),
    ) {
        (Some(proxy_party_1_coin), Some(proxy_party_2_coin)) => {
            // if proxy holds both party contributions, we advance the state machine
            if lp_config.proxy_received_party_contributions(proxy_party_1_coin, proxy_party_2_coin)
            {
                // otherwise we advance the state machine
                CONTRACT_STATE.save(deps.storage, &ContractState::ProxyFunded)?;
                Ok(Response::default()
                    .add_attribute("method", "try_tick")
                    .add_attribute("contract_state", "proxy_funded"))
            } else {
                // otherwise we attempt to deliver the funds
                try_fund_proxy(deps, env)
            }
        }
        // if either balance is unknown, we requery
        _ => {
            // reset the balances and submit the query
            lp_config.reset_latest_proxy_balances();
            LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;
            query_proxy_balances(deps, env)
        }
    }
}

fn try_fund_proxy(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    // we get our target denom balances which we should transfer to the proxy
    let coin_1_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_1_denom_info.local_denom.to_string(),
    )?;

    let coin_2_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_2_denom_info.local_denom.to_string(),
    )?;

    // if either available balance is not sufficient,
    // we reset the latest proxy balance to `None`.
    // this will trigger a query on following tick.
    // TODO: look if this check can cause issues
    if lp_config.party_1_denom_info.osmosis_coin.amount > coin_1_bal.amount
        || lp_config.party_2_denom_info.osmosis_coin.amount > coin_2_bal.amount
    {
        // remove party denom entries from the balances map.
        // this will trigger a proxy balance query on the following tick.
        lp_config.reset_latest_proxy_balances();
        LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

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

fn try_provide_liquidity(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    // we generate a provide_liquidity message for the outpost
    // and wrap it in a note message
    let outpost_msg = lp_config.get_osmo_outpost_provide_liquidity_message()?;
    let note_outpost_liquidity_msg = get_note_execute_neutron_msg(
        vec![outpost_msg],
        ibc_config.osmo_ibc_timeout,
        note_address.clone(),
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&PROVIDE_LIQUIDITY_CALLBACK_ID)?,
        }),
    )?;

    // following the liquidity provision message we perform a proxy balances query.
    // this gets executed after the lp attempt, so on callback we can know if
    // our lp attempt succeeded.
    let note_query_balances_msg = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config.clone(),
        ibc_config,
    )?;

    // reset the prices as they have expired
    lp_config.reset_latest_proxy_balances();
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

    Ok(Response::default()
        .add_message(note_outpost_liquidity_msg)
        .add_message(note_query_balances_msg)
        .add_attribute("method", "try_lp"))
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
        QueryMsg::IbcConfig {} => Ok(to_json_binary(&IBC_CONFIG.may_load(deps.storage)?)?),
        QueryMsg::LiquidityProvisionConfig {} => Ok(to_json_binary(
            &LIQUIDITY_PROVISIONING_CONFIG.may_load(deps.storage)?,
        )?),
        QueryMsg::Callbacks {} => {
            let mut vals = vec![];
            POLYTONE_CALLBACKS
                .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .for_each(|c| {
                    if let Ok((k, v)) = c {
                        vals.push(format!("{:?} : {:?}", k, v))
                    }
                });

            Ok(to_json_binary(&vals)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> NeutronResult<Response> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            holder_address,
            note_address,
            ibc_config,
            lp_config,
        } => {
            let mut response = Response::default().add_attribute("method", "update_config");

            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
                response = response.add_attribute("clock_addr", clock_addr);
            }

            if let Some(holder_address) = holder_address {
                HOLDER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&holder_address)?)?;
                response = response.add_attribute("holder_address", holder_address);
            }

            if let Some(config) = *ibc_config {
                IBC_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            if let Some(address) = note_address {
                let note = deps.api.addr_validate(&address)?;
                NOTE_ADDRESS.save(deps.storage, &note)?;
                response = response.add_attribute("note_address", note);
            }

            if let Some(config) = *lp_config {
                LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
