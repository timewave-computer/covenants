use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_native_splitter::msg::{NativeDenomSplit, PresetNativeSplitterFields, SplitReceiver};
use covenant_single_party_pol_holder::msg::PresetHolderFields;
use covenant_stride_liquid_staker::msg::PresetStrideLsFields;
use covenant_utils::{instantiate2_helper::get_instantiate2_salt_and_address, DestinationConfig};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CONTRACT_CODES, COVENANT_CLOCK_ADDR, HOLDER_ADDR, LIQUID_POOLER_ADDR, LIQUID_STAKER_ADDR,
        LP_FORWARDER_ADDR, LS_FORWARDER_ADDR, ROUTER_ADDR, SPLITTER_ADDR,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-single-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// todo: consider moving these to a getter implemented on
// CovenantContractCodes struct
pub(crate) const CLOCK_SALT: &[u8] = b"clock";
pub(crate) const HOLDER_SALT: &[u8] = b"pol_holder";
pub(crate) const NATIVE_SPLITTER_SALT: &[u8] = b"native_splitter";
pub(crate) const LS_FORWARDER_SALT: &[u8] = b"ls_forwarder";
pub(crate) const LP_FORWARDER_SALT: &[u8] = b"lp_forwarder";
pub(crate) const LIQUID_POOLER_SALT: &[u8] = b"liquid_pooler";
pub(crate) const LIQUID_STAKER_SALT: &[u8] = b"liquid_staker";
pub(crate) const ROUTER_SALT: &[u8] = b"router";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    // todo: return a config with contract code, salt, and address
    let (clock_salt, clock_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let (native_splitter_salt, splitter_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        NATIVE_SPLITTER_SALT,
        &creator_address,
        msg.contract_codes.native_splitter_code,
    )?;
    let (ls_forwarder_salt, ls_forwarder_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LS_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;
    let (lp_forwarder_salt, lp_forwarder_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LP_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;
    let (liquid_staker_salt, liquid_staker_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_STAKER_SALT,
        &creator_address,
        msg.contract_codes.liquid_staker_code,
    )?;
    let (liquid_pooler_salt, liquid_pooler_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_POOLER_SALT,
        &creator_address,
        msg.contract_codes.liquid_pooler_code,
    )?;
    let (holder_salt, holder_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
    )?;
    let (router_salt, router_address) = get_instantiate2_salt_and_address(
        deps.as_ref(),
        ROUTER_SALT,
        &creator_address,
        msg.contract_codes.interchain_router_code,
    )?;

    let mut clock_whitelist = Vec::with_capacity(7);
    clock_whitelist.push(splitter_address.to_string());
    clock_whitelist.push(liquid_pooler_address.to_string());
    clock_whitelist.push(liquid_staker_address.to_string());
    clock_whitelist.push(holder_address.to_string());
    clock_whitelist.push(router_address.to_string());

    let mut denoms: BTreeSet<String> = BTreeSet::new();
    denoms.insert(msg.ls_info.ls_denom_on_neutron.to_string());
    denoms.insert(msg.covenant_party_config.native_denom.to_string());

    let router_instantiate2_msg = PresetInterchainRouterFields {
        destination_config: DestinationConfig {
            local_to_destination_chain_channel_id: msg
                .covenant_party_config
                .host_to_party_chain_channel_id
                .to_string(),
            destination_receiver_addr: msg.covenant_party_config.party_receiver_addr.to_string(),
            ibc_transfer_timeout: msg.covenant_party_config.ibc_transfer_timeout,
            denom_to_pfm_map: msg.pfm_unwinding_config.party_pfm_map.clone(),
        },
        denoms,
        label: format!("{}_interchain_router", msg.label),
        code_id: msg.contract_codes.interchain_router_code,
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        router_salt,
        clock_address.to_string(),
    )?;

    let holder_instantiate2_msg = PresetHolderFields {
        code_id: msg.contract_codes.holder_code,
        label: format!("{}-holder", msg.label),
        withdrawer: Some(msg.covenant_party_config.addr),
        withdraw_to: Some(router_address.to_string()),
        emergency_committee_addr: msg.emergency_committee,
        lockup_period: msg.lockup_period,
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        holder_salt,
        liquid_pooler_address.to_string(),
    )?;

    let liquid_staker_instantiate2_msg = PresetStrideLsFields {
        label: format!("{}_stride_liquid_staker", msg.label),
        ls_denom: msg.ls_info.ls_denom,
        stride_neutron_ibc_transfer_channel_id: msg.ls_info.ls_chain_to_neutron_channel_id,
        neutron_stride_ibc_connection_id: msg.ls_info.ls_neutron_connection_id,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        code_id: msg.contract_codes.liquid_staker_code,
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        liquid_staker_salt,
        clock_address.to_string(),
        liquid_pooler_address.to_string(),
    )?;

    let liquid_pooler_instantiate2_msg = msg.liquid_pooler_config.to_instantiate2_msg(
        env.contract.address.to_string(),
        format!("{}_liquid_pooler", msg.label),
        msg.contract_codes.liquid_pooler_code,
        liquid_pooler_salt,
        clock_address.to_string(),
        holder_address.to_string(),
        msg.pool_price_config,
    )?;

    let splitter_instantiate2_msg = PresetNativeSplitterFields {
        remote_chain_channel_id: msg.native_splitter_config.channel_id,
        remote_chain_connection_id: msg.native_splitter_config.connection_id,
        code_id: msg.contract_codes.native_splitter_code,
        label: format!("{}_remote_chain_splitter", msg.label),
        denom: msg.native_splitter_config.denom.to_string(),
        amount: msg.native_splitter_config.amount,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
    }
    .to_instantiate2_msg(
        env.contract.address.to_string(),
        native_splitter_salt,
        clock_address.to_string(),
        vec![NativeDenomSplit {
            denom: msg.native_splitter_config.denom.to_string(),
            receivers: vec![
                SplitReceiver {
                    addr: ls_forwarder_address.to_string(),
                    share: msg.native_splitter_config.ls_share,
                },
                SplitReceiver {
                    addr: lp_forwarder_address.to_string(),
                    share: msg.native_splitter_config.native_share,
                },
            ],
        }],
    )?;

    let mut messages = vec![
        liquid_staker_instantiate2_msg,
        holder_instantiate2_msg,
        liquid_pooler_instantiate2_msg,
        splitter_instantiate2_msg,
        router_instantiate2_msg,
    ];

    if let CovenantPartyConfig::Interchain(config) = msg.ls_forwarder_config {
        LS_FORWARDER_ADDR.save(deps.storage, &ls_forwarder_address)?;
        clock_whitelist.insert(0, ls_forwarder_address.to_string());
        messages.push(
            PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_ls_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            }
            .to_instantiate2_msg(
                env.contract.address.to_string(),
                ls_forwarder_salt,
                clock_address.to_string(),
                liquid_staker_address.to_string(),
            )?,
        );
    }

    if let CovenantPartyConfig::Interchain(config) = msg.lp_forwarder_config {
        LP_FORWARDER_ADDR.save(deps.storage, &lp_forwarder_address)?;
        clock_whitelist.insert(0, lp_forwarder_address.to_string());
        messages.push(
            PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_lp_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            }
            .to_instantiate2_msg(
                env.contract.address.to_string(),
                lp_forwarder_salt,
                clock_address.to_string(),
                liquid_pooler_address.to_string(),
            )?,
        );
    };

    let clock_instantiate2_msg = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    }
    .to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?;
    messages.insert(0, clock_instantiate2_msg);

    HOLDER_ADDR.save(deps.storage, &holder_address)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_address)?;
    LIQUID_STAKER_ADDR.save(deps.storage, &liquid_staker_address)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_address)?;
    SPLITTER_ADDR.save(deps.storage, &splitter_address)?;
    ROUTER_ADDR.save(deps.storage, &router_address)?;
    LS_FORWARDER_ADDR.save(deps.storage, &ls_forwarder_address)?;
    LP_FORWARDER_ADDR.save(deps.storage, &lp_forwarder_address)?;
    CONTRACT_CODES.save(deps.storage, &msg.contract_codes)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("clock_addr", clock_address)
        .add_attribute("ls_forwarder_addr", ls_forwarder_address)
        .add_attribute("lp_forwarder_addr", lp_forwarder_address)
        .add_attribute("holder_addr", holder_address)
        .add_attribute("splitter_addr", splitter_address)
        .add_attribute("liquid_staker_addr", liquid_staker_address)
        .add_attribute("liquid_pooler_addr", liquid_pooler_address)
        .add_attribute("router_addr", router_address)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(
            &COVENANT_CLOCK_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDR.may_load(deps.storage)?)?),
        QueryMsg::IbcForwarderAddress { ty } => {
            let resp = if ty == "lp" {
                LP_FORWARDER_ADDR.may_load(deps.storage)?
            } else if ty == "ls" {
                LS_FORWARDER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::LiquidStakerAddress {} => {
            Ok(to_json_binary(&LIQUID_STAKER_ADDR.may_load(deps.storage)?)?)
        }
        QueryMsg::LiquidPoolerAddress {} => {
            Ok(to_json_binary(&LIQUID_POOLER_ADDR.may_load(deps.storage)?)?)
        }
        QueryMsg::InterchainRouterAddress {} => {
            Ok(to_json_binary(&ROUTER_ADDR.may_load(deps.storage)?)?)
        }
        QueryMsg::SplitterAddress {} => Ok(to_json_binary(&SPLITTER_ADDR.load(deps.storage)?)?),
        QueryMsg::PartyDepositAddress {} => {
            let splitter_address = SPLITTER_ADDR.load(deps.storage)?;
            let ica: Option<Addr> = deps.querier.query_wasm_smart(
                splitter_address,
                &covenant_utils::neutron_ica::CovenantQueryMsg::DepositAddress {},
            )?;

            Ok(to_json_binary(&ica)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    match msg {
        MigrateMsg::MigrateContracts {
            clock,
            ls_forwarder,
            lp_forwarder,
            holder,
            liquid_pooler,
            liquid_staker,
            splitter,
            router,
        } => {
            let mut migrate_msgs = vec![];
            let mut resp = Response::default().add_attribute("method", "migrate_contracts");
            let contract_codes = CONTRACT_CODES.load(deps.storage)?;

            if let Some(clock) = clock {
                let msg = to_json_binary(&clock)?;
                resp = resp.add_attribute("clock_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.clock_code,
                    msg,
                });
            }

            if let Some(forwarder) = ls_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                resp = resp.add_attribute("ls_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LS_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.ibc_forwarder_code,
                    msg,
                });
            }

            if let Some(forwarder) = lp_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                resp = resp.add_attribute("lp_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LP_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.ibc_forwarder_code,
                    msg,
                });
            }

            if let Some(liquid_pooler) = liquid_pooler {
                let msg: Binary = to_json_binary(&liquid_pooler)?;
                resp = resp.add_attribute("liquid_pooler_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_POOLER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.liquid_pooler_code,
                    msg,
                });
            }

            if let Some(liquid_staker) = liquid_staker {
                let msg: Binary = to_json_binary(&liquid_staker)?;
                resp = resp.add_attribute("liquid_staker_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_STAKER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.liquid_staker_code,
                    msg,
                });
            }

            if let Some(splitter) = splitter {
                let msg: Binary = to_json_binary(&splitter)?;
                resp = resp.add_attribute("splitter_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: SPLITTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.native_splitter_code,
                    msg,
                });
            }

            if let Some(holder) = holder {
                let msg: Binary = to_json_binary(&holder)?;
                resp = resp.add_attribute("holder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.holder_code,
                    msg,
                });
            }

            if let Some(router) = router {
                let msg: Binary = to_json_binary(&router)?;
                resp = resp.add_attribute("router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: contract_codes.interchain_router_code,
                    msg,
                });
            }

            Ok(resp.add_messages(migrate_msgs))
        }
    }
}
