use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};

use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_interchain_splitter::msg::PresetInterchainSplitterFields;
use covenant_swap_holder::msg::PresetSwapHolderFields;
use covenant_utils::{
    instantiate2_helper::get_instantiate2_salt_and_address, CovenantPartiesConfig, CovenantTerms,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, QueryMsg, CovenantContractCodes},
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_INTERCHAIN_SPLITTER_ADDR, COVENANT_SWAP_HOLDER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_ROUTER_ADDR, CONTRACT_CODES,
    },
};

const CONTRACT_NAME: &str = "crates.io:swap-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const CLOCK_SALT: &[u8] = b"clock";
pub(crate) const PARTY_A_ROUTER_SALT: &[u8] = b"party_a_router";
pub(crate) const PARTY_B_ROUTER_SALT: &[u8] = b"party_b_router";
pub(crate) const SPLITTER_SALT: &[u8] = b"splitter";
pub(crate) const HOLDER_SALT: &[u8] = b"holder";
pub(crate) const PARTY_A_FORWARDER_SALT: &[u8] = b"party_a_ibc_forwarder";
pub(crate) const PARTY_B_FORWARDER_SALT: &[u8] = b"party_b_ibc_forwarder";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;


    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let party_a_router_code = msg.party_a_config.get_router_code_id(&msg.contract_codes);
    let party_b_router_code = msg.party_b_config.get_router_code_id(&msg.contract_codes);
    CONTRACT_CODES.save(deps.storage, &msg.contract_codes.to_covenant_codes_config(party_a_router_code, party_b_router_code))?;

    let covenant_denoms: BTreeSet<String> = msg
        .splits
        .iter()
        .map(|split| split.denom.to_string())
        .collect();

    // first we generate the instantiate2 addresses for each contract
    let (party_a_router_salt, party_a_router_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_A_ROUTER_SALT,
        &creator_address,
        party_a_router_code,
    )?;
    let (party_b_router_salt, party_b_router_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_B_ROUTER_SALT,
        &creator_address,
        party_b_router_code,
    )?;
    let (clock_salt, clock_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let (holder_salt, holder_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
    )?;
    let (splitter_salt, splitter_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        SPLITTER_SALT,
        &creator_address,
        msg.contract_codes.splitter_code,
    )?;
    let (party_a_forwarder_salt, party_a_forwarder_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_A_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;
    let (party_b_forwarder_salt, party_b_forwarder_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_B_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;

    let mut clock_whitelist = vec![
        holder_addr.to_string(),
        party_a_router_addr.to_string(),
        party_b_router_addr.to_string(),
        splitter_addr.to_string(),
    ];

    let party_a_router_instantiate2_msg = msg.party_a_config.get_router_instantiate2_wasm_msg(
        format!("{}_party_a_router", msg.label),
        env.contract.address.to_string(),
        clock_addr.to_string(),
        covenant_denoms.clone(),
        party_a_router_code,
        party_a_router_salt,
    )?;
    let party_b_router_instantiate2_msg = msg.party_b_config.get_router_instantiate2_wasm_msg(
        format!("{}_party_b_router", msg.label),
        env.contract.address.to_string(),
        clock_addr.to_string(),
        covenant_denoms.clone(),
        party_b_router_code,
        party_b_router_salt,
    )?;
    let splitter_instantiate2_msg = PresetInterchainSplitterFields {
        splits: msg.clone().splits,
        fallback_split: msg.clone().fallback_split,
        label: format!("{}_interchain_splitter", msg.label),
        code_id: msg.contract_codes.splitter_code,
        party_a_addr: msg.party_a_config.get_final_receiver_address(),
        party_b_addr: msg.party_b_config.get_final_receiver_address(),
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        splitter_salt,
        clock_addr.to_string(),
        party_a_router_addr.to_string(),
        party_b_router_addr.to_string(),
    )?;
    let holder_instantiate2_msg = PresetSwapHolderFields {
        lockup_config: msg.lockup_config,
        parties_config: CovenantPartiesConfig {
            party_a: msg.party_a_config.to_covenant_party(),
            party_b: msg.party_b_config.to_covenant_party(),
        },
        covenant_terms: CovenantTerms::TokenSwap(msg.clone().covenant_terms),
        code_id: msg.contract_codes.holder_code,
        label: format!("{}_swap_holder", msg.label),
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        holder_salt,
        clock_addr.to_string(),
        splitter_addr.to_string(),
    )?;

    let mut messages = vec![
        holder_instantiate2_msg,
        party_a_router_instantiate2_msg,
        party_b_router_instantiate2_msg,
        splitter_instantiate2_msg,
    ];

    // if party A is an interchain party, we include it in the
    // covenant flow. otherwise party is native, meaning that
    // its deposit address will be the holder contract. no
    // extra actions are neeed for that.
    if let CovenantPartyConfig::Interchain(config) = msg.party_a_config {
        // store its forwarder contract address
        PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &party_a_forwarder_addr)?;
        // whitelist that address on the clock
        clock_whitelist.push(party_a_forwarder_addr.to_string());
        // generate its instantiate2 message and add it to the list
        // of instantiation messages
        let ibc_forwarder_instantiate2_msg = PresetIbcForwarderFields {
            remote_chain_connection_id: config.party_chain_connection_id,
            remote_chain_channel_id: config.party_to_host_chain_channel_id,
            denom: config.remote_chain_denom,
            amount: msg.covenant_terms.party_a_amount,
            label: format!("{}_party_a_ibc_forwarder", msg.label),
            code_id: msg.contract_codes.ibc_forwarder_code,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        }
        .to_instantiate2_msg(
            env.contract.address.to_string(),
            party_a_forwarder_salt,
            clock_addr.to_string(),
            holder_addr.to_string(),
        )?;
        messages.push(ibc_forwarder_instantiate2_msg);
    }

    // if party B is an interchain party, we include it in the
    // covenant flow. otherwise party is native, meaning that
    // its deposit address will be the holder contract. no
    // extra actions are neeed for that.
    if let CovenantPartyConfig::Interchain(config) = msg.party_b_config {
        // store its forwarder contract address
        PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_forwarder_addr)?;
        // whitelist that address on the clock
        clock_whitelist.push(party_b_forwarder_addr.to_string());
        // generate its instantiate2 message and add it to the list
        // of instantiation messages
        let ibc_forwarder_instantiate2_msg = PresetIbcForwarderFields {
            remote_chain_connection_id: config.party_chain_connection_id,
            remote_chain_channel_id: config.party_to_host_chain_channel_id,
            denom: config.remote_chain_denom,
            amount: msg.covenant_terms.party_b_amount,
            label: format!("{}_party_b_ibc_forwarder", msg.label),
            code_id: msg.contract_codes.ibc_forwarder_code,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        }
        .to_instantiate2_msg(
            env.contract.address.to_string(),
            party_b_forwarder_salt,
            clock_addr.to_string(),
            holder_addr.to_string(),
        )?;
        messages.push(ibc_forwarder_instantiate2_msg);
    }

    // include the clock in instantiation flow
    messages.insert(
        0,
        PresetClockFields {
            tick_max_gas: msg.clock_tick_max_gas,
            whitelist: clock_whitelist,
            code_id: msg.contract_codes.clock_code,
            label: format!("{}-clock", msg.label),
        }
        .to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?,
    );

    // save the contract addresses
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;
    PARTY_A_ROUTER_ADDR.save(deps.storage, &party_a_router_addr)?;
    PARTY_B_ROUTER_ADDR.save(deps.storage, &party_b_router_addr)?;
    COVENANT_INTERCHAIN_SPLITTER_ADDR.save(deps.storage, &splitter_addr)?;
    COVENANT_SWAP_HOLDER_ADDR.save(deps.storage, &holder_addr)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate_swap_covenant")
        .add_attribute("clock_address", clock_addr.to_string())
        .add_attribute("party_a_router_address", party_a_router_addr.to_string())
        .add_attribute("party_b_router_address", party_b_router_addr.to_string())
        .add_attribute("holder_address", holder_addr.to_string())
        .add_attribute("splitter_address", splitter_addr.to_string())
        .add_attribute(
            "party_a_ibc_forwarder_address",
            party_a_forwarder_addr.to_string(),
        )
        .add_attribute(
            "party_b_ibc_forwarder_address",
            party_b_forwarder_addr.to_string(),
        )
        .add_attribute("instantiation_messages", to_json_string(&messages)?)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(
            &COVENANT_CLOCK_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(
            &COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::SplitterAddress {} => Ok(to_json_binary(
            &COVENANT_INTERCHAIN_SPLITTER_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::InterchainRouterAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_ROUTER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_ROUTER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::IbcForwarderAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::PartyDepositAddress { party } => {
            // here depending on the party we query their ibc forwarder.
            // if it's present, we then query it for a deposit address
            // which should return the address of ICA on a remote chain.
            // if no ibc forwarder is saved, we return the holder.
            let resp = if party == "party_a" {
                match PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else if party == "party_b" {
                match PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
    }
}

// TODO: add migrations
