#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};

use covenant_astroport_liquid_pooler::msg::{
    AssetData, PresetAstroLiquidPoolerFields, SingleSideLpLimits,
};
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_native_splitter::msg::{NativeDenomSplit, PresetNativeSplitterFields, SplitReceiver};
use covenant_single_party_pol_holder::msg::PresetHolderFields;
use covenant_stride_liquid_staker::msg::PresetStrideLsFields;
use covenant_utils::instantiate2_helper::get_instantiate2_salt_and_address;
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, HOLDER_ADDR, LIQUID_POOLER_ADDR, LIQUID_STAKER_ADDR,
        LP_FORWARDER_ADDR, LS_FORWARDER_ADDR, PRESET_CLOCK_FIELDS, PRESET_HOLDER_FIELDS,
        PRESET_LIQUID_POOLER_FIELDS, PRESET_LIQUID_STAKER_FIELDS, PRESET_LP_FORWARDER_FIELDS,
        PRESET_LS_FORWARDER_FIELDS, PRESET_SPLITTER_FIELDS, SPLITTER_ADDR,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-single-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_SALT: &[u8] = b"clock";
pub const HOLDER_SALT: &[u8] = b"pol_holder";
pub const NATIVE_SPLITTER_SALT: &[u8] = b"native_splitter";

pub const LS_FORWARDER_SALT: &[u8] = b"ls_forwarder";
pub const LP_FORWARDER_SALT: &[u8] = b"lp_forwarder";

pub const LIQUID_POOLER_SALT: &[u8] = b"liquid_pooler";
pub const LIQUID_STAKER_SALT: &[u8] = b"liquid_staker";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;

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

    HOLDER_ADDR.save(deps.storage, &holder_address)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_address)?;
    LIQUID_STAKER_ADDR.save(deps.storage, &liquid_staker_address)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_address)?;
    SPLITTER_ADDR.save(deps.storage, &splitter_address)?;

    let mut clock_whitelist = Vec::with_capacity(7);
    clock_whitelist.push(splitter_address.to_string());
    clock_whitelist.push(liquid_pooler_address.to_string());
    clock_whitelist.push(liquid_staker_address.to_string());
    clock_whitelist.push(holder_address.to_string());

    let preset_ls_forwarder_fields = match msg.clone().ls_forwarder_config {
        CovenantPartyConfig::Interchain(config) => {
            LS_FORWARDER_ADDR.save(deps.storage, &ls_forwarder_address)?;
            clock_whitelist.insert(0, ls_forwarder_address.to_string());

            let preset = PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_ls_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            };
            PRESET_LS_FORWARDER_FIELDS.save(deps.storage, &preset)?;

            Some(preset)
        }
        CovenantPartyConfig::Native(_) => None,
    };

    let preset_lp_forwarder_fields = match msg.clone().lp_forwarder_config {
        CovenantPartyConfig::Interchain(config) => {
            LP_FORWARDER_ADDR.save(deps.storage, &lp_forwarder_address)?;
            clock_whitelist.insert(0, lp_forwarder_address.to_string());

            let preset = PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_lp_ibc_forwarder", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            };
            PRESET_LP_FORWARDER_FIELDS.save(deps.storage, &preset)?;

            Some(preset)
        }
        CovenantPartyConfig::Native(_) => None,
    };

    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };
    PRESET_CLOCK_FIELDS.save(deps.storage, &preset_clock_fields)?;

    // Holder
    let preset_holder_fields = PresetHolderFields {
        code_id: msg.contract_codes.holder_code,
        label: format!("{}-holder", msg.label),
        withdrawer: msg.withdrawer,
        withdraw_to: msg.withdraw_to,
        emergency_committee_addr: msg.emerrgency_committee,
        lockup_period: msg.lockup_period,
    };
    PRESET_HOLDER_FIELDS.save(deps.storage, &preset_holder_fields)?;

    // Liquid staker
    let preset_liquid_staker_fields = PresetStrideLsFields {
        label: format!("{}_stride_liquid_staker", msg.label),
        ls_denom: msg.ls_info.ls_denom,
        stride_neutron_ibc_transfer_channel_id: msg.ls_info.ls_chain_to_neutron_channel_id,
        neutron_stride_ibc_connection_id: msg.ls_info.ls_neutron_connection_id,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        code_id: msg.contract_codes.liquid_staker_code,
    };
    PRESET_LIQUID_STAKER_FIELDS.save(deps.storage, &preset_liquid_staker_fields)?;

    // Liquid pooler
    let preset_liquid_pooler_fields = PresetAstroLiquidPoolerFields {
        slippage_tolerance: None,
        assets: AssetData {
            asset_a_denom: msg.ls_info.ls_denom_on_neutron,
            asset_b_denom: msg.lp_forwarder_config.get_native_denom(),
        },
        single_side_lp_limits: SingleSideLpLimits {
            asset_a_limit: msg.party_a_single_side_limit,
            asset_b_limit: msg.party_b_single_side_limit,
        },
        label: format!("{}_liquid_pooler", msg.label),
        code_id: msg.contract_codes.liquid_pooler_code,
        expected_pool_ratio: msg.expected_pool_ratio,
        acceptable_pool_ratio_delta: msg.acceptable_pool_ratio_delta,
        pair_type: msg.pool_pair_type,
    };
    PRESET_LIQUID_POOLER_FIELDS.save(deps.storage, &preset_liquid_pooler_fields)?;

    let preset_splitter_fields = PresetNativeSplitterFields {
        remote_chain_channel_id: msg.native_splitter_config.channel_id,
        remote_chain_connection_id: msg.native_splitter_config.connection_id,
        code_id: msg.contract_codes.native_splitter_code,
        label: format!("{}_remote_chain_splitter", msg.label),
        denom: msg.native_splitter_config.denom.to_string(),
        amount: msg.native_splitter_config.amount,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
    };
    PRESET_SPLITTER_FIELDS.save(deps.storage, &preset_splitter_fields)?;

    let mut messages = vec![
        preset_clock_fields.to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?,
        preset_liquid_staker_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            liquid_staker_salt,
            clock_address.to_string(),
            liquid_pooler_address.to_string(),
        )?,
        preset_holder_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            holder_salt,
            liquid_pooler_address.to_string(),
        )?,
        preset_liquid_pooler_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            liquid_pooler_salt,
            msg.pool_address,
            clock_address.to_string(),
            holder_address.to_string(),
        )?,
        preset_splitter_fields.to_instantiate2_msg(
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
        )?,
    ];

    if let Some(fields) = preset_ls_forwarder_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            ls_forwarder_salt,
            clock_address.to_string(),
            liquid_staker_address.to_string(),
        )?);
    }

    if let Some(fields) = preset_lp_forwarder_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            lp_forwarder_salt,
            clock_address.to_string(),
            liquid_pooler_address.to_string(),
        )?);
    };

    Ok(Response::default()
        .add_messages(messages)
        .add_attribute("method", "instantiate"))
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
            liquid_staker,
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

            if let Some(forwarder) = ls_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_LS_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("ls_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LS_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = lp_forwarder {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_LP_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("lp_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LP_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(liquid_pooler) = liquid_pooler {
                let msg: Binary = to_json_binary(&liquid_pooler)?;
                let liquid_pooler_fields = PRESET_LIQUID_POOLER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("liquid_pooler_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_POOLER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: liquid_pooler_fields.code_id,
                    msg,
                });
            }

            if let Some(liquid_staker) = liquid_staker {
                let msg: Binary = to_json_binary(&liquid_staker)?;
                let liquid_staker_fields = PRESET_LIQUID_STAKER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("liquid_staker_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_STAKER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: liquid_staker_fields.code_id,
                    msg,
                });
            }

            if let Some(splitter) = splitter {
                let msg: Binary = to_json_binary(&splitter)?;
                let splitter_fields = PRESET_SPLITTER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("splitter_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: SPLITTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: splitter_fields.code_id,
                    msg,
                });
            }

            if let Some(holder) = holder {
                let msg: Binary = to_json_binary(&holder)?;
                let holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("holder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: holder_fields.code_id,
                    msg,
                });
            }

            if let Some(liquid_staker) = liquid_staker {
                let msg: Binary = to_json_binary(&liquid_staker)?;
                let liquid_staker_fields = PRESET_LIQUID_STAKER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("liquid_staker_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: LIQUID_STAKER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: liquid_staker_fields.code_id,
                    msg,
                });
            }

            Ok(resp.add_messages(migrate_msgs))
        }
    }
}
