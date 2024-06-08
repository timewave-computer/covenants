use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, to_json_string, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, WasmMsg,
};
use covenant_utils::{
    instantiate2_helper::get_instantiate2_salt_and_address, op_mode::ContractOperationModeConfig,
    split::remap_splits, CovenantPartiesConfig, CovenantTerms, SwapCovenantTerms,
};
use cw2::set_contract_version;
use valence_swap_holder::msg::RefundConfig;

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg, RouterMigrateMsg},
    state::{
        CONTRACT_CODES, COVENANT_CLOCK_ADDR, COVENANT_INTERCHAIN_SPLITTER_ADDR,
        COVENANT_SWAP_HOLDER_ADDR, PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR,
        PARTY_B_IBC_FORWARDER_ADDR, PARTY_B_ROUTER_ADDR,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut resp = Response::default().add_attribute("method", "instantiate_swap_covenant");

    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let covenant_denoms: BTreeSet<String> = msg.splits.keys().map(|k| k.to_string()).collect();

    // first we generate the instantiate2 addresses for each contract
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
    let clock_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let holder_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
    )?;
    let splitter_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        SPLITTER_SALT,
        &creator_address,
        msg.contract_codes.splitter_code,
    )?;

    CONTRACT_CODES.save(
        deps.storage,
        &msg.contract_codes.to_covenant_codes_config(
            party_a_router_instantiate2_config.code,
            party_b_router_instantiate2_config.code,
        ),
    )?;

    let mut clock_whitelist = vec![
        holder_instantiate2_config.addr.to_string(),
        splitter_instantiate2_config.addr.to_string(),
    ];

    let mut clock_initial_queue = vec![];

    // Note: Native Router has privileged_accounts, Interchain Router doesn't yet ..
    // TODO: when both native router & interchain router have privileged_accounts, we can remove this match,
    // and just add both router addresses to the clock_initial_queue.
    match msg.party_a_config {
        CovenantPartyConfig::Native(_) => {
            clock_initial_queue.push(party_a_router_instantiate2_config.addr.to_string())
        }
        CovenantPartyConfig::Interchain(_) => {
            clock_whitelist.push(party_a_router_instantiate2_config.addr.to_string())
        }
    }
    match msg.party_b_config {
        CovenantPartyConfig::Native(_) => {
            clock_initial_queue.push(party_b_router_instantiate2_config.addr.to_string())
        }
        CovenantPartyConfig::Interchain(_) => {
            clock_whitelist.push(party_b_router_instantiate2_config.addr.to_string())
        }
    }

    let party_a_router_instantiate2_msg = msg.party_a_config.get_router_instantiate2_wasm_msg(
        format!("{}_party_a_router", msg.label),
        env.contract.address.to_string(),
        clock_instantiate2_config.addr.clone(),
        covenant_denoms.clone(),
        party_a_router_instantiate2_config.clone(),
    )?;
    let party_b_router_instantiate2_msg = msg.party_b_config.get_router_instantiate2_wasm_msg(
        format!("{}_party_b_router", msg.label),
        env.contract.address.to_string(),
        clock_instantiate2_config.addr.clone(),
        covenant_denoms.clone(),
        party_b_router_instantiate2_config.clone(),
    )?;

    // we validate that denoms explicitly defined in splits are the
    // same denoms that parties are expected to contribute
    ensure!(
        msg.splits
            .contains_key(&msg.party_a_config.get_native_denom()),
        ContractError::DenomMisconfigurationError(
            msg.party_a_config.get_native_denom(),
            format!("{:?}", covenant_denoms)
        )
    );
    ensure!(
        msg.splits
            .contains_key(&msg.party_b_config.get_native_denom()),
        ContractError::DenomMisconfigurationError(
            msg.party_b_config.get_native_denom(),
            format!("{:?}", covenant_denoms)
        )
    );

    let splitter_instantiate2_msg = valence_native_splitter::msg::InstantiateMsg {
        clock_address: clock_instantiate2_config.addr.to_string(),
        splits: remap_splits(
            msg.splits.clone(),
            (
                msg.party_a_config.get_final_receiver_address(),
                party_a_router_instantiate2_config.addr.to_string(),
            ),
            (
                msg.party_b_config.get_final_receiver_address(),
                party_b_router_instantiate2_config.addr.to_string(),
            ),
        )?,
        fallback_split: match msg.fallback_split.clone() {
            Some(config) => Some(config.remap_receivers_to_routers(
                msg.party_a_config.get_final_receiver_address(),
                party_a_router_instantiate2_config.addr.to_string(),
                msg.party_b_config.get_final_receiver_address(),
                party_b_router_instantiate2_config.addr.to_string(),
            )?),
            None => None,
        },
    }
    .to_instantiate2_msg(
        &splitter_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_interchain_splitter", msg.label),
    )?;

    let holder_instantiate2_msg = valence_swap_holder::msg::InstantiateMsg {
        lockup_config: msg.lockup_config,
        parties_config: CovenantPartiesConfig {
            party_a: msg.party_a_config.to_covenant_party(),
            party_b: msg.party_b_config.to_covenant_party(),
        },
        covenant_terms: CovenantTerms::TokenSwap(SwapCovenantTerms {
            party_a_amount: msg.party_a_config.get_contribution().amount,
            party_b_amount: msg.party_b_config.get_contribution().amount,
        }),
        op_mode_cfg: ContractOperationModeConfig::Permissioned(vec![clock_instantiate2_config
            .addr
            .to_string()]),
        next_contract: splitter_instantiate2_config.addr.to_string(),
        refund_config: RefundConfig {
            party_a_refund_address: party_a_router_instantiate2_config.addr.to_string(),
            party_b_refund_address: party_b_router_instantiate2_config.addr.to_string(),
        },
    }
    .to_instantiate2_msg(
        &holder_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_swap_holder", msg.label),
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
    if let CovenantPartyConfig::Interchain(config) = &msg.party_a_config {
        let party_a_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
            deps.as_ref(),
            PARTY_A_FORWARDER_SALT,
            &creator_address,
            msg.contract_codes.ibc_forwarder_code,
        )?;
        // store its forwarder contract address
        PARTY_A_IBC_FORWARDER_ADDR
            .save(deps.storage, &party_a_forwarder_instantiate2_config.addr)?;
        // Add that address to the clock's initial queue param
        clock_initial_queue.push(party_a_forwarder_instantiate2_config.addr.to_string());
        // generate its instantiate2 message and add it to the list
        // of instantiation messages
        let instantiate_msg = valence_ibc_forwarder::msg::InstantiateMsg {
            remote_chain_connection_id: config.party_chain_connection_id.to_string(),
            remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
            denom: config.remote_chain_denom.to_string(),
            amount: msg.party_a_config.get_contribution().amount,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            op_mode_cfg: ContractOperationModeConfig::Permissioned(vec![clock_instantiate2_config
                .addr
                .to_string()]),
            next_contract: holder_instantiate2_config.addr.to_string(),
            fallback_address: msg.fallback_address.clone(),
        }
        .to_instantiate2_msg(
            &party_a_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_party_a_ibc_forwarder", msg.label),
        )?;
        messages.push(instantiate_msg);
        resp = resp.add_attribute(
            "party_a_ibc_forwarder_address",
            party_a_forwarder_instantiate2_config.addr.to_string(),
        );
    }

    // if party B is an interchain party, we include it in the
    // covenant flow. otherwise party is native, meaning that
    // its deposit address will be the holder contract. no
    // extra actions are neeed for that.
    if let CovenantPartyConfig::Interchain(config) = &msg.party_b_config {
        let party_b_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
            deps.as_ref(),
            PARTY_B_FORWARDER_SALT,
            &creator_address,
            msg.contract_codes.ibc_forwarder_code,
        )?;
        // store its forwarder contract address
        PARTY_B_IBC_FORWARDER_ADDR
            .save(deps.storage, &party_b_forwarder_instantiate2_config.addr)?;
        // Add that address to the clock's initial queue param
        clock_initial_queue.push(party_b_forwarder_instantiate2_config.addr.to_string());
        // generate its instantiate2 message and add it to the list
        // of instantiation messages
        let instantiate_msg = valence_ibc_forwarder::msg::InstantiateMsg {
            remote_chain_connection_id: config.party_chain_connection_id.to_string(),
            remote_chain_channel_id: config.party_to_host_chain_channel_id.to_string(),
            denom: config.remote_chain_denom.to_string(),
            amount: msg.party_b_config.get_contribution().amount,
            ica_timeout: msg.timeouts.ica_timeout,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            op_mode_cfg: ContractOperationModeConfig::Permissioned(vec![clock_instantiate2_config
                .addr
                .to_string()]),
            next_contract: holder_instantiate2_config.addr.to_string(),
            fallback_address: msg.fallback_address,
        }
        .to_instantiate2_msg(
            &party_b_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_party_b_ibc_forwarder", msg.label),
        )?;
        messages.push(instantiate_msg);
        resp = resp.add_attribute(
            "party_b_ibc_forwarder_address",
            party_b_forwarder_instantiate2_config.addr.to_string(),
        );
    }

    // include the clock in instantiation flow
    messages.insert(
        0,
        valence_clock::msg::InstantiateMsg {
            tick_max_gas: msg.clock_tick_max_gas,
            whitelist: clock_whitelist,
            initial_queue: clock_initial_queue,
        }
        .to_instantiate2_msg(
            clock_instantiate2_config.code,
            clock_instantiate2_config.salt,
            env.contract.address.to_string(),
            format!("{}-clock", msg.label),
        )?,
    );

    // save the contract addresses
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_instantiate2_config.addr)?;
    PARTY_A_ROUTER_ADDR.save(deps.storage, &party_a_router_instantiate2_config.addr)?;
    PARTY_B_ROUTER_ADDR.save(deps.storage, &party_b_router_instantiate2_config.addr)?;
    COVENANT_INTERCHAIN_SPLITTER_ADDR.save(deps.storage, &splitter_instantiate2_config.addr)?;
    COVENANT_SWAP_HOLDER_ADDR.save(deps.storage, &holder_instantiate2_config.addr)?;

    Ok(resp
        .add_attribute("clock_address", clock_instantiate2_config.addr.to_string())
        .add_attribute(
            "party_a_router_address",
            party_a_router_instantiate2_config.addr.to_string(),
        )
        .add_attribute(
            "party_b_router_address",
            party_b_router_instantiate2_config.addr.to_string(),
        )
        .add_attribute(
            "holder_address",
            holder_instantiate2_config.addr.to_string(),
        )
        .add_attribute(
            "splitter_address",
            splitter_instantiate2_config.addr.to_string(),
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
                return Err(StdError::not_found("unknown party"));
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::IbcForwarderAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else {
                return Err(StdError::not_found("unknown party"));
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
                        &covenant_utils::neutron::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else if party == "party_b" {
                match PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else {
                return Err(StdError::not_found("unknown party"));
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
            splitter,
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

            if let Some(forwarder) = *party_a_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                resp = resp.add_attribute("party_a_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.party_a_forwarder,
                    msg,
                });
            }

            if let Some(forwarder) = *party_b_forwarder {
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
                    contract_addr: COVENANT_SWAP_HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.holder,
                    msg,
                });
            }

            if let Some(splitter) = splitter {
                let msg = to_json_binary(&splitter)?;
                resp = resp.add_attribute("splitter_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_INTERCHAIN_SPLITTER_ADDR
                        .load(deps.storage)?
                        .to_string(),
                    new_code_id: contract_codes.splitter,
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
