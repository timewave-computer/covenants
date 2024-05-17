use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CanonicalAddr, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, WasmMsg,
};
use covenant_utils::{instantiate2_helper::get_instantiate2_salt_and_address, split::remap_splits};
use cw2::set_contract_version;
use valence_ibc_forwarder::msg::InstantiateMsg as IbcForwarderInstantiateMsg;
use valence_two_party_pol_holder::msg::{RagequitConfig, TwoPartyPolCovenantConfig};

use crate::{
    error::ContractError,
    msg::{
        CovenantPartyConfig, InstantiateMsg, LiquidPoolerMigrateMsg, MigrateMsg, QueryMsg,
        RouterMigrateMsg,
    },
    state::{
        CONTRACT_CODES, COVENANT_CLOCK_ADDR, COVENANT_POL_HOLDER_ADDR, LIQUID_POOLER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_ROUTER_ADDR,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_SALT: &[u8] = b"clock";
pub const PARTY_A_ROUTER_SALT: &[u8] = b"router_a";
pub const PARTY_B_ROUTER_SALT: &[u8] = b"router_b";
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
    let mut resp = Response::default().add_attribute("method", "instantiate");
    let creator_address: CanonicalAddr =
        deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let covenant_denoms: BTreeSet<String> = msg.splits.keys().map(|k| k.to_string()).collect();

    let clock_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let party_a_router_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_A_ROUTER_SALT,
        &creator_address,
        msg.party_a_config.get_router_code_id(&msg.contract_codes),
    )?;
    let party_b_router_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        PARTY_B_ROUTER_SALT,
        &creator_address,
        msg.party_b_config.get_router_code_id(&msg.contract_codes),
    )?;
    let holder_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
    )?;
    let liquid_pooler_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_POOLER_SALT,
        &creator_address,
        msg.contract_codes.liquid_pooler_code,
    )?;

    let mut clock_whitelist: Vec<String> = Vec::with_capacity(6);
    clock_whitelist.push(holder_instantiate2_config.addr.to_string());
    clock_whitelist.push(party_a_router_instantiate2_config.addr.to_string());
    clock_whitelist.push(party_b_router_instantiate2_config.addr.to_string());
    clock_whitelist.push(liquid_pooler_instantiate2_config.addr.to_string());

    let mut clock_initial_queue = vec![];

    let holder_instantiate2_msg = valence_two_party_pol_holder::msg::InstantiateMsg {
        clock_address: clock_instantiate2_config.addr.to_string(),
        lockup_config: msg.lockup_config,
        next_contract: liquid_pooler_instantiate2_config.addr.to_string(),
        ragequit_config: msg.ragequit_config.unwrap_or(RagequitConfig::Disabled),
        deposit_deadline: msg.deposit_deadline,
        splits: remap_splits(
            msg.splits,
            (
                msg.party_a_config.get_final_receiver_address(),
                party_a_router_instantiate2_config.addr.to_string(),
            ),
            (
                msg.party_b_config.get_final_receiver_address(),
                party_b_router_instantiate2_config.addr.to_string(),
            ),
        )?,
        fallback_split: match msg.fallback_split {
            Some(config) => Some(config.remap_receivers_to_routers(
                msg.party_a_config.get_final_receiver_address(),
                party_a_router_instantiate2_config.addr.to_string(),
                msg.party_b_config.get_final_receiver_address(),
                party_b_router_instantiate2_config.addr.to_string(),
            )?),
            None => None,
        },
        covenant_config: TwoPartyPolCovenantConfig {
            party_a: msg.party_a_config.to_two_party_pol_party(
                msg.party_a_share,
                party_a_router_instantiate2_config.addr.to_string(),
            ),
            party_b: msg.party_b_config.to_two_party_pol_party(
                msg.party_b_share,
                party_b_router_instantiate2_config.addr.to_string(),
            ),
            covenant_type: msg.covenant_type.clone(),
        },
        emergency_committee_addr: msg.emergency_committee,
    }
    .to_instantiate2_msg(
        &holder_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_holder", msg.label),
    )?;

    let party_a_router_instantiate2_msg = msg.party_a_config.to_router_instantiate2_msg(
        env.contract.address.to_string(),
        clock_instantiate2_config.addr.clone(),
        format!("{}_party_a_router", msg.label),
        covenant_denoms.clone(),
        party_a_router_instantiate2_config.clone(),
    )?;

    let party_b_router_instantiate2_msg = msg.party_b_config.to_router_instantiate2_msg(
        env.contract.address.to_string(),
        clock_instantiate2_config.addr.clone(),
        format!("{}_party_b_router", msg.label),
        covenant_denoms.clone(),
        party_b_router_instantiate2_config.clone(),
    )?;

    let liquid_pooler_instantiate2_msg = msg.liquid_pooler_config.to_instantiate2_msg(
        &liquid_pooler_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_liquid_pooler", msg.label),
        clock_instantiate2_config.addr.to_string(),
        holder_instantiate2_config.addr.to_string(),
        msg.pool_price_config,
    )?;

    let mut messages = vec![
        holder_instantiate2_msg,
        party_a_router_instantiate2_msg,
        party_b_router_instantiate2_msg,
        liquid_pooler_instantiate2_msg,
    ];

    if let CovenantPartyConfig::Interchain(config) = &msg.party_a_config {
        let party_a_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
            deps.as_ref(),
            PARTY_A_FORWARDER_SALT,
            &creator_address,
            msg.contract_codes.ibc_forwarder_code,
        )?;
        PARTY_A_IBC_FORWARDER_ADDR
            .save(deps.storage, &party_a_forwarder_instantiate2_config.addr)?;
        clock_initial_queue.push(party_a_forwarder_instantiate2_config.addr.to_string());
        let instantiate_msg = IbcForwarderInstantiateMsg {
            privileged_accounts: Some(vec![clock_instantiate2_config.addr.to_string()]),
            next_contract: holder_instantiate2_config.addr.to_string(),
            remote_chain_connection_id: config.party_chain_connection_id.to_string(),
            remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
            denom: config.remote_chain_denom.to_string(),
            amount: config.contribution.amount,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            fallback_address: msg.fallback_address.clone(),
        };

        messages.push(instantiate_msg.to_instantiate2_msg(
            &party_a_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_party_a_ibc_forwarder", msg.label),
        )?);
        resp = resp.add_attribute(
            "party_a_forwarder_addr",
            party_a_forwarder_instantiate2_config.addr,
        );
    }

    if let CovenantPartyConfig::Interchain(config) = &msg.party_b_config {
        let party_b_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
            deps.as_ref(),
            PARTY_B_FORWARDER_SALT,
            &creator_address,
            msg.contract_codes.ibc_forwarder_code,
        )?;
        PARTY_B_IBC_FORWARDER_ADDR
            .save(deps.storage, &party_b_forwarder_instantiate2_config.addr)?;
        clock_initial_queue.push(party_b_forwarder_instantiate2_config.addr.to_string());
        let instantiate_msg = IbcForwarderInstantiateMsg {
            privileged_accounts: Some(vec![clock_instantiate2_config.addr.to_string()]),
            next_contract: holder_instantiate2_config.addr.to_string(),
            remote_chain_connection_id: config.party_chain_connection_id.to_string(),
            remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
            denom: config.remote_chain_denom.to_string(),
            amount: config.contribution.amount,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            fallback_address: msg.fallback_address,
        };

        messages.push(instantiate_msg.to_instantiate2_msg(
            &party_b_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_party_b_ibc_forwarder", msg.label),
        )?);
        resp = resp.add_attribute(
            "party_b_forwarder_addr",
            party_b_forwarder_instantiate2_config.addr,
        );
    }

    let clock_instantiate2_msg = valence_clock::msg::InstantiateMsg {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
        initial_queue: clock_initial_queue,
    }
    .to_instantiate2_msg(
        clock_instantiate2_config.code,
        clock_instantiate2_config.salt,
        env.contract.address.to_string(),
        format!("{}-clock", msg.label),
    )?;
    messages.insert(0, clock_instantiate2_msg);

    CONTRACT_CODES.save(
        deps.storage,
        &msg.contract_codes.to_covenant_codes_config(
            party_a_router_instantiate2_config.code,
            party_b_router_instantiate2_config.code,
        ),
    )?;
    COVENANT_POL_HOLDER_ADDR.save(deps.storage, &holder_instantiate2_config.addr)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_instantiate2_config.addr)?;
    PARTY_B_ROUTER_ADDR.save(deps.storage, &party_b_router_instantiate2_config.addr)?;
    PARTY_A_ROUTER_ADDR.save(deps.storage, &party_a_router_instantiate2_config.addr)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_instantiate2_config.addr)?;

    Ok(resp
        .add_attribute("clock_addr", clock_instantiate2_config.addr)
        .add_attribute("liquid_pooler_addr", liquid_pooler_instantiate2_config.addr)
        .add_attribute(
            "party_a_router_addr",
            party_a_router_instantiate2_config.addr,
        )
        .add_attribute(
            "party_b_router_addr",
            party_b_router_instantiate2_config.addr,
        )
        .add_attribute("holder_addr", holder_instantiate2_config.addr)
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
                return Err(StdError::not_found("not found"));
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::InterchainRouterAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_ROUTER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_ROUTER_ADDR.may_load(deps.storage)?
            } else {
                return Err(StdError::not_found("not found"));
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
                        &covenant_utils::neutron::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_POL_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else if party == "party_b" {
                match PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_POL_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else {
                return Err(StdError::not_found("not found"));
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::ContractCodes {} => Ok(to_json_binary(&CONTRACT_CODES.load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::UpdateCovenant {
            codes,
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

            if let Some(new_codes) = codes {
                CONTRACT_CODES.save(deps.storage, &new_codes)?;
                let code_binary = to_json_binary(&new_codes)?;
                resp = resp.add_attribute("contract_codes_migrate", code_binary.to_base64());
            }

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

            if let Some(router_migrate_msg) = party_a_router {
                let msg: Binary = match router_migrate_msg {
                    RouterMigrateMsg::Interchain(msg) => to_json_binary(&msg)?,
                    RouterMigrateMsg::Native(msg) => to_json_binary(&msg)?,
                };
                resp = resp.add_attribute("party_a_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_a_router,
                    msg,
                });
            }

            if let Some(router_migrate_msg) = party_b_router {
                let msg: Binary = match router_migrate_msg {
                    RouterMigrateMsg::Interchain(msg) => to_json_binary(&msg)?,
                    RouterMigrateMsg::Native(msg) => to_json_binary(&msg)?,
                };
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

            if let Some(liquid_pooler_migrate_msg) = liquid_pooler {
                let msg: Binary = match liquid_pooler_migrate_msg {
                    LiquidPoolerMigrateMsg::Astroport(msg) => to_json_binary(&msg)?,
                    LiquidPoolerMigrateMsg::Osmosis(msg) => to_json_binary(&msg)?,
                };

                resp = resp.add_attribute("liquid_pooler_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_POOLER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.liquid_pooler,
                    msg,
                });
            }

            Ok(resp.add_messages(migrate_msgs))
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
