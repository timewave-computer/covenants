#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, SubMsg, Attribute,
};
use covenant_clock::helpers::verify_clock;
use covenant_utils::neutron_ica::{SudoPayload, RemoteChainInfo};
use cw2::set_contract_version;

use crate::msg::{
    ContractState, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{
    save_reply_payload, CLOCK_ADDRESS, CONTRACT_STATE,
    REMOTE_CHAIN_INFO, SPLIT_CONFIG_MAP,
};
use neutron_sdk::{
    bindings::{
        msg::NeutronMsg,
        query::NeutronQuery,
    },
    NeutronResult,
};


const CONTRACT_NAME: &str = "crates.io:covenant-native-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUDO_PAYLOAD_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    let remote_chain_info = RemoteChainInfo {
        connection_id: msg.remote_chain_connection_id,
        channel_id: msg.remote_chain_channel_id,
        denom: msg.denom,
        ibc_transfer_timeout: msg.ibc_transfer_timeout,
        ica_timeout: msg.ica_timeout,
        ibc_fee: msg.ibc_fee,
    };
    REMOTE_CHAIN_INFO.save(deps.storage, &remote_chain_info)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // validate each split and store it in a map
    let mut split_resp_attributes: Vec<Attribute> = Vec::new();
    for split in msg.splits {
        let validated_split = split.validate()?;
        split_resp_attributes.push(validated_split.to_response_attribute());
        SPLIT_CONFIG_MAP.save(deps.storage, validated_split.denom, &validated_split.receivers)?;
    }

    Ok(Response::default()
        .add_attribute("method", "native_splitter_instantiate")
        .add_attribute("clock_address", clock_addr)
        .add_attributes(remote_chain_info.get_response_attributes())
        .add_attributes(split_resp_attributes)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {msg:?}").as_str());
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => Ok(Response::default()),
        ContractState::IcaCreated => Ok(Response::default()),
        ContractState::Completed => Ok(Response::default()),
    }
}

#[allow(unused)]
fn msg_with_sudo_callback<C: Into<CosmosMsg<T>>, T>(
    deps: DepsMut,
    msg: C,
    payload: SudoPayload,
) -> StdResult<SubMsg<T>> {
    save_reply_payload(deps.storage, payload)?;
    Ok(SubMsg::reply_on_success(msg, SUDO_PAYLOAD_REPLY_ID))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => {
            Ok(to_binary(&Some(1))?)
        },
        QueryMsg::RemoteChainInfo {} => Ok(to_binary(&REMOTE_CHAIN_INFO.may_load(deps.storage)?)?),
    }
}
