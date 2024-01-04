use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg, WasmMsg, QueryRequest, Empty, StdError, to_json_string, Coin, IbcTimeout, IbcMsg, SubMsg, Uint64,
};
use covenant_utils::{get_polytone_execute_msg_binary, query_polytone_proxy_address, get_polytone_query_msg_binary, default_ibc_fee};
use cw2::set_contract_version;
use neutron_sdk::{bindings::msg::NeutronMsg, sudo::msg::RequestPacketTimeoutHeight, NeutronResult};
use osmosis_std::types::{
    osmosis::gamm::v1beta1::{MsgJoinPool, Pool},
    cosmos::base::v1beta1::Coin as OsmosisCoin,
};

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, ProvidedLiquidityInfo, QueryMsg, PacketMetadata, ForwardMetadata, PartyChainInfo},
    state::{HOLDER_ADDRESS, PROVIDED_LIQUIDITY_INFO, NOTE_ADDRESS, PROXY_ADDRESS, CALLBACKS, LATEST_OSMO_POOL_SNAPSHOT, POOL_ID, PARTY_1_CHAIN_INFO, PARTY_2_CHAIN_INFO, OSMO_TO_NEUTRON_CHANNEL_ID, LATEST_PROXY_BALANCES, PARTY_1_DENOM_INFO, PARTY_2_DENOM_INFO, OSMOSIS_IBC_TIMEOUT}, polytone_handlers::{process_fatal_error_callback, process_execute_callback, process_query_callback},
};

use polytone::callbacks::{Callback as PolytoneCallback, CallbackMessage, CallbackRequest};

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
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;
    let note_addr = deps.api.addr_validate(&msg.note_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant module addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;

    NOTE_ADDRESS.save(deps.storage, &note_addr)?;
    CALLBACKS.save(deps.storage, &Vec::new())?;
    LATEST_OSMO_POOL_SNAPSHOT.save(deps.storage, &None)?;
    LATEST_PROXY_BALANCES.save(deps.storage, &None)?;
    POOL_ID.save(deps.storage, &msg.pool_id)?;
    OSMOSIS_IBC_TIMEOUT.save(deps.storage, &msg.osmo_ibc_timeout)?;
    PARTY_1_CHAIN_INFO.save(deps.storage, &msg.party_1_chain_info)?;
    PARTY_2_CHAIN_INFO.save(deps.storage, &msg.party_2_chain_info)?;
    OSMO_TO_NEUTRON_CHANNEL_ID.save(deps.storage, &msg.osmo_to_neutron_channel_id)?;

    PARTY_1_DENOM_INFO.save(deps.storage, &msg.party_1_denom_info)?;
    PARTY_2_DENOM_INFO.save(deps.storage, &msg.party_2_denom_info)?;

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
        ExecuteMsg::Callback(
            callback_msg
        ) => try_handle_callback(deps, info, callback_msg),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_handle_callback(deps: DepsMut, info: MessageInfo, msg: CallbackMessage) -> NeutronResult<Response<NeutronMsg>> {
    // only the note can submit a callback
    if info.sender != NOTE_ADDRESS.load(deps.storage)? {
        return Err(ContractError::Unauthorized {}.to_neutron_std())
    }

    match msg.result {
        PolytoneCallback::Query(resp) =>
            process_query_callback(deps,resp, msg.initiator_msg),
        PolytoneCallback::Execute(resp) =>
            process_execute_callback(deps,resp, msg.initiator_msg),
        PolytoneCallback::FatalError(resp) =>
            process_fatal_error_callback(deps, resp),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    // verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        // create a proxy account
        ContractState::Instantiated => try_query_proxy_address(deps, env),
        // fund the proxy account
        ContractState::ProxyCreated => {
            match LATEST_PROXY_BALANCES.load(deps.storage)? {
                // if no balances are stored, we query them
                None => query_proxy_balances(deps, env),
                // otherwise we attempt to advance
                Some(balances) => {
                    // we validate the proxy balances
                    let party_1_denom_info = PARTY_1_DENOM_INFO.load(deps.storage)?;
                    let party_2_denom_info = PARTY_2_DENOM_INFO.load(deps.storage)?;

                    // we assume coins are not funded and try to prove otherwise
                    let mut coin_1_funded = false;
                    let mut coin_2_funded = false;
                    balances.iter()
                        .for_each(|b| {
                            if b.denom == party_1_denom_info.osmosis_coin.denom
                            && b.amount >= party_1_denom_info.osmosis_coin.amount {
                                coin_1_funded = true;
                            }
                            else if b.denom == party_2_denom_info.osmosis_coin.denom
                            && b.amount >= party_2_denom_info.osmosis_coin.amount {
                                coin_2_funded = true;
                            }
                        });

                    // if either coin is not funded, we attempt to do so
                    if !coin_1_funded || !coin_2_funded {
                        // and try to fund the proxy
                        try_fund_proxy(deps, env)
                    } else {
                        // otherwise we advance the state machine
                        CONTRACT_STATE.save(deps.storage, &ContractState::ProxyFunded)?;
                        Ok(Response::default()
                            .add_attribute("method", "try_tick")
                            .add_attribute("contract_state", "proxy_funded"))
                    }
                },

            }
        },
        // attempt to provide liquidity
        ContractState::ProxyFunded => {
            match LATEST_OSMO_POOL_SNAPSHOT.load(deps.storage)? {
                // if no pool snapshot is available, we query it.
                // snapshots get reset to `None` after a successful
                // `try_lp` call in order to refresh the price
                // before attempting to provide liquidity again.
                None => try_query_pool(deps, env),
                // if pool snapshot is available, we use it for LP attempt
                Some(pool) => try_lp(deps, env, pool),
            }
        },
        // no longer accept any actions
        ContractState::Complete => Err(ContractError::StateMachineError("complete".to_string()).to_neutron_std()),
    }
}

fn query_proxy_balances(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let osmo_ibc_timeout = OSMOSIS_IBC_TIMEOUT.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    let party_1_denom_info = PARTY_1_DENOM_INFO.load(deps.storage)?;
    let party_2_denom_info = PARTY_2_DENOM_INFO.load(deps.storage)?;


    let proxy_coin_1_balance_request: QueryRequest<Empty> = osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest {
        address: proxy_address.to_string(),
        denom: party_1_denom_info.osmosis_coin.denom,
    }
    .into();
    let proxy_coin_2_balance_request: QueryRequest<Empty> = osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest {
        address: proxy_address,
        denom: party_2_denom_info.osmosis_coin.denom,
    }
    .into();

    let polytone_query_msg_binary = get_polytone_query_msg_binary(
        vec![proxy_coin_1_balance_request, proxy_coin_2_balance_request],
        CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&"proxy_balances")?,
        },
        osmo_ibc_timeout,
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
        .add_attribute("method", "try_query_proxy_balances"))
}

fn try_query_pool(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {

    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let pool_id: u64 = POOL_ID.load(deps.storage)?.u64();
    let osmo_ibc_timeout = OSMOSIS_IBC_TIMEOUT.load(deps.storage)?;

    let query_pool_request: QueryRequest<Empty> = osmosis_std::types::osmosis::gamm::v1beta1::QueryPoolRequest {
        pool_id,
    }
    .into();

    let polytone_query_msg_binary = get_polytone_query_msg_binary(
        vec![query_pool_request],
        CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&"query_pool")?,
        },
        osmo_ibc_timeout,
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


fn try_create_proxy(deps: DepsMut, env: Env, note_address: String) -> NeutronResult<Response<NeutronMsg>> {
    let osmo_ibc_timeout = OSMOSIS_IBC_TIMEOUT.load(deps.storage)?;
    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary("proxy_created")?,
        }),
        osmo_ibc_timeout,
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
        },
        // if proxy is not created, try to create it
        None => try_create_proxy(deps, env, note_address.to_string()),
    }
}

fn try_fund_proxy(deps: DepsMut, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let party_1_denom_info = PARTY_1_DENOM_INFO.load(deps.storage)?;
    let party_2_denom_info = PARTY_2_DENOM_INFO.load(deps.storage)?;

    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let party_1_chain_info = PARTY_1_CHAIN_INFO.load(deps.storage)?;
    let party_2_chain_info = PARTY_2_CHAIN_INFO.load(deps.storage)?;

    let coin_1_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        party_1_denom_info.neutron_denom,
    )?;

    let coin_2_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        party_2_denom_info.neutron_denom,
    )?;

    // if either available balance is not sufficient,
    // we reset the latest proxy balance to `None`.
    // this will trigger a query on following tick.
    if party_1_denom_info.osmosis_coin.amount > coin_1_bal.amount || party_2_denom_info.osmosis_coin.amount > coin_2_bal.amount {
        // first we reset the latest proxy balance to `None`.
        // this will trigger a query on following tick.
        LATEST_PROXY_BALANCES.save(deps.storage, &None)?;
        return Ok(Response::default()
            .add_attribute("method", "try_fund_proxy")
            .add_attribute("result", "insufficient_balances"))
    }

    let party_1_transfer_msg = get_ibc_transfer_message(
        party_1_chain_info,
        env.clone(),
        coin_1_bal,
        proxy_address.to_string(),
    )?;

    let party_2_transfer_msg = get_ibc_transfer_message(
        party_2_chain_info,
        env,
        coin_2_bal,
        proxy_address,
    )?;

    Ok(Response::default()
        .add_message(CosmosMsg::Custom(party_1_transfer_msg))
        .add_message(CosmosMsg::Custom(party_2_transfer_msg))
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
            timeout_height: RequestPacketTimeoutHeight { revision_number: None, revision_height: None },
            timeout_timestamp: env.block.time.plus_seconds(party_chain_info.ibc_timeout.u64()).nanos(),
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
            timeout_height: RequestPacketTimeoutHeight { revision_number: None, revision_height: None },
            timeout_timestamp: env.block.time.plus_seconds(party_chain_info.ibc_timeout.u64()).nanos(),
            memo: "".to_string(),
            fee: default_ibc_fee(),
        }),
    }
}

fn try_lp(deps: DepsMut, env: Env, pool: Pool) -> NeutronResult<Response<NeutronMsg>> {
    // we invalidate the latest pool snapshot to force it to be queried
    // for subsequent lp attempts
    LATEST_OSMO_POOL_SNAPSHOT.save(deps.storage, &None)?;

    // this call means proxy is created, funded, and we are ready to LP
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let pool_id: u64 = POOL_ID.load(deps.storage)?.u64();
    let osmo_ibc_timeout = OSMOSIS_IBC_TIMEOUT.load(deps.storage)?;

    let party_1_denom_info = PARTY_1_DENOM_INFO.load(deps.storage)?;
    let party_2_denom_info = PARTY_2_DENOM_INFO.load(deps.storage)?;



    let token_in_maxs: Vec<OsmosisCoin> = vec![
        party_1_denom_info.osmosis_coin.clone().into(),
        party_2_denom_info.osmosis_coin.clone().into()];

    let gamm_shares_coin = match pool.total_shares {
        Some(coin) => coin,
        None => return Err(ContractError::OsmosisPoolError(
            "expected Some(total_shares), found None".to_string()
        ).to_neutron_std()),
    };

    let pool_assets: Vec<OsmosisCoin> = pool.pool_assets.into_iter()
        .filter_map(|asset| asset.token)
        .collect();

    let (pool_asset_1_amount, pool_asset_2_amount) = match (pool_assets.get(0), pool_assets.get(1)) {
        (Some(pool_asset_1), Some(pool_asset_2)) => {
            if pool_asset_1.denom == party_1_denom_info.osmosis_coin.denom
            && pool_asset_2.denom == party_2_denom_info.osmosis_coin.denom {
                (pool_asset_1.amount.to_string(), pool_asset_2.amount.to_string())
            } else {
                (pool_asset_2.amount.to_string(), pool_asset_1.amount.to_string())
            }
        },
        _ => return Err(ContractError::OsmosisPoolError(
            "osmosis pool assets mismatch".to_string()
        ).to_neutron_std()),
    };

    let share_out_amount = std::cmp::min(
        party_1_denom_info.osmosis_coin.amount.multiply_ratio(
            Uint128::from_str(&gamm_shares_coin.amount)?,
            Uint128::from_str(&pool_asset_1_amount)?.u128(),
        ),
        party_2_denom_info.osmosis_coin.amount.multiply_ratio(
            Uint128::from_str(&gamm_shares_coin.amount)?,
            Uint128::from_str(&pool_asset_2_amount)?.u128(),
        ),
    );

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: proxy_address,
        pool_id,
        // exact number of shares we wish to receive
        share_out_amount: share_out_amount.to_string(),
        token_in_maxs,
    }
    .into();
    let expected_gamm_coin = Coin {
        denom: gamm_shares_coin.denom,
        amount: share_out_amount,
    };
    let holder_addr = HOLDER_ADDRESS.load(deps.storage)?;
    let osmo_to_neutron_channel_id = OSMO_TO_NEUTRON_CHANNEL_ID.load(deps.storage)?;
    let transfer_gamm_msg: CosmosMsg = IbcMsg::Transfer {
        channel_id: osmo_to_neutron_channel_id,
        to_address: holder_addr.to_string(),
        amount: expected_gamm_coin.clone(),
        timeout: IbcTimeout::with_timestamp(
            env.block.time.plus_seconds(osmo_ibc_timeout.u64()),
        ),
    }.into();
    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        // include gamm token transfer msg back to holder after osmo_msg
        vec![osmo_msg, transfer_gamm_msg],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&"liquidity_provided")?,
        }),
        osmo_ibc_timeout,
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
            Ok(to_json_binary(&LATEST_OSMO_POOL_SNAPSHOT.load(deps.storage)?)?)
        }
    }
}
