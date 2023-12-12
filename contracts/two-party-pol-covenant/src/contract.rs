use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CanonicalAddr, CodeInfoResponse, Coin,
    Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use covenant_astroport_liquid_pooler::msg::{
    AssetData, PresetAstroLiquidPoolerFields, SingleSideLpLimits,
};
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_two_party_pol_holder::msg::{
    PresetPolParty, PresetTwoPartyPolHolderFields, RagequitConfig,
};
use covenant_utils::{DestinationConfig, ReceiverConfig};
use cw2::set_contract_version;
use sha2::{Digest, Sha256};

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_POL_HOLDER_ADDR, LIQUID_POOLER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_ROUTER_ADDR, PRESET_CLOCK_FIELDS, PRESET_HOLDER_FIELDS,
        PRESET_PARTY_A_FORWARDER_FIELDS, PRESET_PARTY_A_ROUTER_FIELDS,
        PRESET_PARTY_B_FORWARDER_FIELDS, PRESET_PARTY_B_ROUTER_FIELDS,
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

fn get_precomputed_address(
    deps: Deps,
    code_id: u64,
    creator: &CanonicalAddr,
    salt: &[u8],
) -> Result<Addr, ContractError> {
    let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;

    let precomputed_address = instantiate2_address(&checksum, creator, salt)?;

    Ok(deps.api.addr_humanize(&precomputed_address)?)
}

pub fn generate_contract_salt(salt_str: &[u8]) -> cosmwasm_std::Binary {
    let mut hasher = Sha256::new();
    hasher.update(salt_str);
    hasher.finalize().to_vec().into()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let clock_salt = generate_contract_salt(CLOCK_SALT);
    let party_a_router_salt = generate_contract_salt(PARTY_A_INTERCHAIN_ROUTER_SALT);
    let party_b_router_salt = generate_contract_salt(PARTY_B_INTERCHAIN_ROUTER_SALT);
    let holder_salt = generate_contract_salt(HOLDER_SALT);
    let party_a_forwarder_salt = generate_contract_salt(PARTY_A_FORWARDER_SALT);
    let party_b_forwarder_salt = generate_contract_salt(PARTY_B_FORWARDER_SALT);
    let liquid_pooler_salt = generate_contract_salt(LIQUID_POOLER_SALT);

    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let clock_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.clock_code,
        &creator_address,
        &clock_salt,
    )?;
    let party_a_interchain_router_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.router_code,
        &creator_address,
        &party_a_router_salt,
    )?;
    let party_b_interchain_router_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.router_code,
        &creator_address,
        &party_b_router_salt,
    )?;
    let liquid_pooler_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.liquid_pooler_code,
        &creator_address,
        &liquid_pooler_salt,
    )?;
    let holder_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.holder_code,
        &creator_address,
        &holder_salt,
    )?;
    let party_a_forwarder_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.ibc_forwarder_code,
        &creator_address,
        &party_a_forwarder_salt,
    )?;
    let party_b_forwarder_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.ibc_forwarder_code,
        &creator_address,
        &party_b_forwarder_salt,
    )?;

    PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_forwarder_address)?;
    PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &party_a_forwarder_address)?;
    COVENANT_POL_HOLDER_ADDR.save(deps.storage, &holder_address)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_address)?;
    PARTY_B_ROUTER_ADDR.save(deps.storage, &party_b_interchain_router_address)?;
    PARTY_A_ROUTER_ADDR.save(deps.storage, &party_a_interchain_router_address)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_address)?;

    let covenant_denoms: BTreeSet<String> = msg
        .splits
        .iter()
        .map(|split| split.denom.to_string())
        .collect();

    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: vec![
            party_a_forwarder_address.to_string(),
            party_b_forwarder_address.to_string(),
            holder_address.to_string(),
            party_a_interchain_router_address.to_string(),
            party_b_interchain_router_address.to_string(),
            liquid_pooler_address.to_string(),
        ],
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };
    let preset_holder_fields = PresetTwoPartyPolHolderFields {
        lockup_config: msg.lockup_config,
        pool_address: msg.pool_address,
        ragequit_config: msg.ragequit_config.unwrap_or(RagequitConfig::Disabled),
        deposit_deadline: msg.deposit_deadline,
        party_a: msg.party_a_config.to_preset_pol_party(msg.party_a_share),
        party_b: msg
            .party_b_config
            .clone()
            .to_preset_pol_party(msg.party_b_share),
        code_id: msg.contract_codes.holder_code,
        label: format!("{}-holder", msg.label),
        splits: msg.splits,
        fallback_split: msg.fallback_split,
        covenant_type: msg.covenant_type,
    };

    let preset_party_a_router_fields = PresetInterchainRouterFields {
        receiver_config: ReceiverConfig::Ibc(DestinationConfig {
            destination_chain_channel_id: msg.party_a_config.host_to_party_chain_channel_id,
            destination_receiver_addr: msg.party_a_config.controller_addr,
            ibc_transfer_timeout: msg.party_a_config.ibc_transfer_timeout,
        }),
        label: format!("{}_party_a_interchain_router", msg.label),
        code_id: msg.contract_codes.router_code,
        denoms: covenant_denoms.clone(),
    };
    let preset_party_b_router_fields = PresetInterchainRouterFields {
        receiver_config: ReceiverConfig::Ibc(DestinationConfig {
            destination_chain_channel_id: msg.party_b_config.host_to_party_chain_channel_id,
            destination_receiver_addr: msg.party_b_config.controller_addr,
            ibc_transfer_timeout: msg.party_b_config.ibc_transfer_timeout,
        }),
        label: format!("{}_party_b_interchain_router", msg.label),
        code_id: msg.contract_codes.router_code,
        denoms: covenant_denoms,
    };

    let preset_liquid_pooler_fields = PresetAstroLiquidPoolerFields {
        slippage_tolerance: None,
        assets: AssetData {
            asset_a_denom: msg.party_a_config.get_native_denom(),
            asset_b_denom: msg.party_b_config.get_native_denom(),
        },
        single_side_lp_limits: SingleSideLpLimits {
            asset_a_limit: Uint128::new(10000),
            asset_b_limit: Uint128::new(100000),
        },
        label: format!("{}_liquid_pooler", msg.label),
        code_id: msg.contract_codes.liquid_pooler_code,
        expected_pool_ratio: msg.expected_pool_ratio,
        acceptable_pool_ratio_delta: msg.acceptable_pool_ratio_delta,
        pair_type: msg.pool_pair_type,
    };
    let mut messages = vec![
        preset_clock_fields.to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?,
        preset_holder_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            holder_salt,
            clock_address.to_string(),
            liquid_pooler_address.to_string(),
            party_a_router_address.to_string(),
            party_b_router_address.to_string(),
        )?,
        preset_party_a_router_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            party_a_router_salt,
            clock_address.to_string(),
        )?,
        preset_party_b_router_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            party_b_router_salt,
            clock_address.to_string(),
        )?,
        preset_liquid_pooler_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            liquid_pooler_salt,
            preset_holder_fields.pool_address,
            clock_address.to_string(),
            holder_address.to_string(),
        )?,
    ];

    let clock_instantiate2_msg =
        preset_clock_fields.to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?;

    let holder_instantiate2_msg = preset_holder_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        holder_salt,
        clock_address.to_string(),
        liquid_pooler_address.to_string(),
        party_a_interchain_router_address.to_string(),
        party_b_interchain_router_address.to_string(),
    )?;

    let party_a_router_instantiate2_msg = preset_party_a_router_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_a_router_salt,
        clock_address.to_string(),
    )?;

    let party_b_router_instantiate2_msg = preset_party_b_router_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_b_router_salt,
        clock_address.to_string(),
    )?;

    let liquid_pooler_instantiate2_msg = preset_liquid_pooler_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        liquid_pooler_salt,
        preset_holder_fields.pool_address,
        clock_address.to_string(),
        holder_address.to_string(),
    )?;

    let party_a_ibc_forwarder_instantiate2_msg = preset_party_a_forwarder_fields
        .to_instantiate2_msg(
            env.contract.address.to_string(),
            party_a_forwarder_salt,
            clock_address.to_string(),
            holder_address.to_string(),
        )?;

    let party_b_ibc_forwarder_instantiate2_msg = preset_party_b_forwarder_fields
        .to_instantiate2_msg(
            env.contract.address.to_string(),
            party_b_forwarder_salt,
            clock_address.to_string(),
            holder_address.to_string(),
        )?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_messages(vec![
            clock_instantiate2_msg,
            party_a_router_instantiate2_msg,
            party_b_router_instantiate2_msg,
            liquid_pooler_instantiate2_msg,
            holder_instantiate2_msg,
            party_a_ibc_forwarder_instantiate2_msg,
            party_b_ibc_forwarder_instantiate2_msg,
        ]))
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
        MigrateMsg::MigrateContracts {
            clock,
            holder,
            party_a_router,
            party_b_router,
            party_a_forwarder,
            party_b_forwarder,
        } => {
            let mut migrate_msgs = vec![];
            let mut resp = Response::default().add_attribute("method", "migrate_contracts");

            if let Some(clock) = clock {
                let msg = to_json_binary(&clock)?;
                let clock_fields = PRESET_CLOCK_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("clock_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: clock_fields.code_id,
                    msg,
                });
            }

            if let Some(router) = party_a_router {
                let msg: Binary = to_json_binary(&router)?;
                let router_fields = PRESET_PARTY_A_ROUTER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_a_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: router_fields.code_id,
                    msg,
                });
            }

            if let Some(router) = party_b_router {
                let msg: Binary = to_json_binary(&router)?;
                let router_fields = PRESET_PARTY_B_ROUTER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_b_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: router_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = party_a_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_PARTY_A_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_a_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = party_b_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_PARTY_B_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_b_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(holder) = holder {
                let msg: Binary = to_json_binary(&holder)?;
                let holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("holder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_POL_HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: holder_fields.code_id,
                    msg,
                });
            }

            Ok(resp.add_messages(migrate_msgs))
        }
    }
}
