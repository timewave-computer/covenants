use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CanonicalAddr, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, WasmMsg,
};
use covenant_utils::instantiate2_helper::get_instantiate2_salt_and_address;
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_two_party_pol_holder::msg::{PresetTwoPartyPolHolderFields, RagequitConfig};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_POL_HOLDER_ADDR, LIQUID_POOLER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_ROUTER_ADDR, CONTRACT_CODES,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-two-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_SALT: &[u8] = b"clock";
pub const PARTY_A_INTERCHAIN_ROUTER_SALT: &[u8] = b"router_a";
pub const PARTY_B_INTERCHAIN_ROUTER_SALT: &[u8] = b"router_b";
pub const HOLDER_SALT: &[u8] = b"pol_holder";
pub const PARTY_A_FORWARDER_SALT: &[u8] = b"forwarder_a";
pub const PARTY_B_FORWARDER_SALT: &[u8] = b"forwarder_b";
pub const LIQUID_POOLER_SALT: &[u8] = b"liquid_pooler";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let creator_address: CanonicalAddr =
        deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let party_a_router_code = msg.party_a_config.get_router_code_id(&msg.contract_codes);
    let party_b_router_code = msg.party_b_config.get_router_code_id(&msg.contract_codes);

    CONTRACT_CODES.save(deps.storage, &msg.contract_codes.to_covenant_codes_config(party_a_router_code, party_b_router_code))?;
    let covenant_denoms: BTreeSet<String> = msg
        .splits
        .iter()
        .map(|split| split.denom.to_string())
        .collect();

    let (clock_salt, clock_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let (party_a_router_salt, party_a_router_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_A_INTERCHAIN_ROUTER_SALT,
        &creator_address,
        party_a_router_code,
    )?;
    let (party_b_router_salt, party_b_router_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_B_INTERCHAIN_ROUTER_SALT,
        &creator_address,
        party_b_router_code,
    )?;
    let (holder_salt, holder_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
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
    let (liquid_pooler_salt, liquid_pooler_addr) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_POOLER_SALT,
        &creator_address,
        msg.contract_codes.liquid_pooler_code,
    )?;

    let mut clock_whitelist: Vec<String> = Vec::with_capacity(6);

    let preset_party_a_forwarder_fields =
        if let CovenantPartyConfig::Interchain(config) = &msg.party_a_config {
            PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &party_a_forwarder_addr)?;
            clock_whitelist.push(party_a_forwarder_addr.to_string());
            Some(PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id.to_string(),
                remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
                denom: config.remote_chain_denom.to_string(),
                amount: config.contribution.amount,
                label: format!("{}_party_a_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            })
        } else {
            None
        };

    let preset_party_b_forwarder_fields =
        if let CovenantPartyConfig::Interchain(config) = &msg.party_b_config {
            PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_forwarder_addr)?;
            clock_whitelist.push(party_b_forwarder_addr.to_string());
            Some(PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id.to_string(),
                remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
                denom: config.remote_chain_denom.to_string(),
                amount: config.contribution.amount,
                label: format!("{}_party_b_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            })
        } else {
            None
        };

    clock_whitelist.push(holder_addr.to_string());
    clock_whitelist.push(party_a_router_addr.to_string());
    clock_whitelist.push(party_b_router_addr.to_string());
    clock_whitelist.push(liquid_pooler_addr.to_string());

    let clock_instantiate2_msg = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    }
    .to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?;

    let holder_instantiate2_msg = PresetTwoPartyPolHolderFields {
        lockup_config: msg.lockup_config,
        ragequit_config: msg.ragequit_config.unwrap_or(RagequitConfig::Disabled),
        deposit_deadline: msg.deposit_deadline,
        party_a: msg.party_a_config.to_preset_pol_party(msg.party_a_share),
        party_b: msg.party_b_config.to_preset_pol_party(msg.party_b_share),
        code_id: msg.contract_codes.holder_code,
        label: format!("{}-holder", msg.label),
        splits: msg.splits,
        fallback_split: msg.fallback_split,
        covenant_type: msg.covenant_type,
        emergency_committee: msg.emergency_committee,
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        holder_salt,
        clock_addr.to_string(),
        liquid_pooler_addr.to_string(),
        party_a_router_addr.to_string(),
        party_b_router_addr.to_string(),
    )?;

    let party_a_router_instantiate2_msg = msg.party_a_config.to_router_instantiate2_msg(
        env.contract.address.to_string(),
        clock_addr.to_string(),
        party_a_router_salt,
        party_a_router_code,
        format!("{}_party_a_router", msg.label),
        covenant_denoms.clone(),
        msg.pfm_unwinding_config.party_1_pfm_map,
    )?;

    let party_b_router_instantiate2_msg = msg.party_b_config.to_router_instantiate2_msg(
        env.contract.address.to_string(),
        clock_addr.to_string(),
        party_b_router_salt,
        party_b_router_code,
        format!("{}_party_b_router", msg.label),
        covenant_denoms.clone(),
        msg.pfm_unwinding_config.party_2_pfm_map,
    )?;

    let liquid_pooler_instantiate2_msg = msg.liquid_pooler_config.to_instantiate2_msg(
        env.contract.address.to_string(),
        format!("{}_liquid_pooler", msg.label),
        msg.contract_codes.liquid_pooler_code,
        liquid_pooler_salt,
        clock_addr.to_string(),
        holder_addr.to_string(),
        (msg.expected_pool_ratio, msg.acceptable_pool_ratio_delta),
    )?;

    let mut messages = vec![
        clock_instantiate2_msg,
        holder_instantiate2_msg,
        party_a_router_instantiate2_msg,
        party_b_router_instantiate2_msg,
        liquid_pooler_instantiate2_msg,
    ];

    if let Some(fields) = preset_party_a_forwarder_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            party_a_forwarder_salt,
            clock_addr.to_string(),
            holder_addr.to_string(),
        )?);
    }

    if let Some(fields) = preset_party_b_forwarder_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            party_b_forwarder_salt,
            clock_addr.to_string(),
            holder_addr.to_string(),
        )?);
    };

    COVENANT_POL_HOLDER_ADDR.save(deps.storage, &holder_addr)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_addr)?;
    PARTY_B_ROUTER_ADDR.save(deps.storage, &party_b_router_addr)?;
    PARTY_A_ROUTER_ADDR.save(deps.storage, &party_a_router_addr)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("clock_addr", clock_addr)
        .add_attribute("liquid_pooler_addr", liquid_pooler_addr)
        .add_attribute("party_a_router_addr", party_a_router_addr)
        .add_attribute("party_b_router_addr", party_b_router_addr)
        .add_attribute("holder_addr", holder_addr)
        .add_attribute("party_a_forwarder_addr", party_a_forwarder_addr)
        .add_attribute("party_b_forwarder_addr", party_b_forwarder_addr)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(
            &COVENANT_CLOCK_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(
            &COVENANT_POL_HOLDER_ADDR.may_load(deps.storage)?,
        )?),
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
        QueryMsg::LiquidPoolerAddress {} => {
            Ok(to_json_binary(&LIQUID_POOLER_ADDR.may_load(deps.storage)?)?)
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
                    None => COVENANT_POL_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else if party == "party_b" {
                match PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_POL_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    match msg {
        MigrateMsg::UpdateCovenant {
            clock,
            holder,
            liquid_pooler,
            party_a_router,
            party_b_router,
            party_a_forwarder,
            party_b_forwarder,
        } => {
            let mut migrate_msgs = vec![];
            let mut resp = Response::default().add_attribute("method", "migrate_contracts");
            let contract_codes = CONTRACT_CODES.load(deps.storage)?;

            if let Some(clock) = clock {
                let msg = to_json_binary(&clock)?;
                resp = resp.add_attribute("clock_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.clock,
                    msg,
                });
            }

            if let Some(router) = party_a_router {
                let msg: Binary = to_json_binary(&router)?;
                resp = resp.add_attribute("party_a_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_a_router,
                    msg,
                });
            }

            if let Some(router) = party_b_router {
                let msg: Binary = to_json_binary(&router)?;
                resp = resp.add_attribute("party_b_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_b_router,
                    msg,
                });
            }

            if let Some(forwarder) = party_a_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                resp = resp.add_attribute("party_a_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_a_forwarder,
                    msg,
                });
            }

            if let Some(forwarder) = party_b_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                resp = resp.add_attribute("party_b_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_b_forwarder,
                    msg,
                });
            }

            if let Some(holder) = holder {
                let msg: Binary = to_json_binary(&holder)?;
                resp = resp.add_attribute("holder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_POL_HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.holder,
                    msg,
                });
            }

            if let Some(liquid_pooler) = liquid_pooler {
                let msg = to_json_binary(&liquid_pooler)?;
                resp = resp.add_attribute("liquid_pooler_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_POOLER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.liquid_pooler,
                    msg,
                });
            }

            Ok(resp.add_messages(migrate_msgs))
        }
    }
}
