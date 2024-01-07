#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CanonicalAddr, CodeInfoResponse, Deps,
    DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use covenant_astroport_liquid_pooler::msg::{
    AssetData, PresetAstroLiquidPoolerFields, SingleSideLpLimits,
};
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_stride_liquid_staker::msg::PresetStrideLsFields;
use cw2::set_contract_version;
use sha2::{Digest, Sha256};

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, HOLDER_ADDR, IBC_FORWARDER_A_ADDR, IBC_FORWARDER_B_ADDR,
        LIQUID_POOLER_ADDR, LIQUID_STAKER_ADDR, PRESET_CLOCK_FIELDS, PRESET_FORWARDER_A_FIELDS,
        PRESET_FORWARDER_B_FIELDS, PRESET_LIQUID_POOLER_FIELDS, PRESET_LIQUID_STAKER_FIELDS,
        PRESET_SPLITTER_FIELDS, SPLITTER_ADDR,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-two-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_SALT: &[u8] = b"clock";
pub const HOLDER_SALT: &[u8] = b"pol_holder";
pub const NATIVE_SPLITTER: &[u8] = b"native_splitter";
pub const FORWARDER_A_SALT: &[u8] = b"forwarder_a";
pub const FORWARDER_B_SALT: &[u8] = b"forwarder_b";
pub const LIQUID_POOLER_SALT: &[u8] = b"liquid_pooler";
pub const LIQUID_STAKER_SALT: &[u8] = b"liquid_staker";

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
    let native_splitter_salt = generate_contract_salt(NATIVE_SPLITTER);
    let forwarder_a_salt = generate_contract_salt(FORWARDER_A_SALT);
    let forwarder_b_salt = generate_contract_salt(FORWARDER_B_SALT);
    let liquid_staker_salt = generate_contract_salt(LIQUID_STAKER_SALT);
    let liquid_pooler_salt = generate_contract_salt(LIQUID_POOLER_SALT);
    let holder_salt = generate_contract_salt(HOLDER_SALT);

    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let clock_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.clock_code,
        &creator_address,
        &clock_salt,
    )?;

    let splitter_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.native_splitter_code,
        &creator_address,
        &native_splitter_salt,
    )?;

    let forwarder_a_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.ibc_forwarder_code,
        &creator_address,
        &forwarder_a_salt,
    )?;

    let forwarder_b_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.ibc_forwarder_code,
        &creator_address,
        &forwarder_b_salt,
    )?;

    let liquid_staker_address = get_precomputed_address(
        deps.as_ref(),
        msg.contract_codes.liquid_staker_code,
        &creator_address,
        &liquid_staker_salt,
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

    HOLDER_ADDR.save(deps.storage, &holder_address)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_address)?;
    LIQUID_STAKER_ADDR.save(deps.storage, &liquid_staker_address)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_address)?;
    SPLITTER_ADDR.save(deps.storage, &splitter_address)?;

    let mut clock_whitelist = Vec::with_capacity(6);

    let preset_forwarder_a_fields = match msg.clone().forwarder_a_config {
        CovenantPartyConfig::Interchain(config) => {
            IBC_FORWARDER_A_ADDR.save(deps.storage, &forwarder_a_address)?;
            clock_whitelist.push(forwarder_a_address.to_string());

            let preset = PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_ibc_forwarder_a", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            };
            PRESET_FORWARDER_B_FIELDS.save(deps.storage, &preset)?;

            Some(preset)
        }
        CovenantPartyConfig::Native(_) => None,
    };

    let preset_forwarder_b_fields = match msg.clone().forwarder_b_config {
        CovenantPartyConfig::Interchain(config) => {
            IBC_FORWARDER_B_ADDR.save(deps.storage, &forwarder_b_address)?;
            clock_whitelist.push(forwarder_b_address.to_string());

            let preset = PresetIbcForwarderFields {
                remote_chain_connection_id: config.party_chain_connection_id,
                remote_chain_channel_id: config.party_to_host_chain_channel_id,
                denom: config.remote_chain_denom,
                amount: config.contribution.amount,
                label: format!("{}_ibc_forwarder_b", msg.label),
                code_id: msg.contract_codes.ibc_forwarder_code,
                ica_timeout: msg.timeouts.ica_timeout,
                ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
                ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
            };
            PRESET_FORWARDER_A_FIELDS.save(deps.storage, &preset)?;

            Some(preset)
        }
        CovenantPartyConfig::Native(_) => None,
    };

    clock_whitelist.push(liquid_pooler_address.to_string());
    clock_whitelist.push(liquid_staker_address.to_string());
    clock_whitelist.push(splitter_address.to_string());
    clock_whitelist.push(holder_address.to_string());

    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };
    PRESET_CLOCK_FIELDS.save(deps.storage, &preset_clock_fields)?;

    // TODO: Holder
    // let preset_holder_fields = PresetTwoPartyPolHolderFields {
    //     lockup_config: msg.lockup_config,
    //     pool_address: msg.pool_address,
    //     ragequit_config: msg.ragequit_config.unwrap_or(RagequitConfig::Disabled),
    //     deposit_deadline: msg.deposit_deadline,
    //     party_a: msg.party_a_config.to_preset_pol_party(msg.party_a_share),
    //     party_b: msg
    //         .party_b_config
    //         .clone()
    //         .to_preset_pol_party(msg.party_b_share),
    //     code_id: msg.contract_codes.holder_code,
    //     label: format!("{}-holder", msg.label),
    //     splits: msg.splits,
    //     fallback_split: msg.fallback_split,
    //     covenant_type: msg.covenant_type,
    // };
    // PRESET_HOLDER_FIELDS.save(deps.storage, &preset_holder_fields)?;

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
            asset_b_denom: msg.forwarder_b_config.get_native_denom(),
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
    PRESET_LIQUID_POOLER_FIELDS.save(deps.storage, &preset_liquid_pooler_fields)?;

    let mut messages = vec![
        preset_clock_fields.to_instantiate2_msg(env.contract.address.to_string(), clock_salt)?,
        preset_liquid_staker_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            liquid_staker_salt,
            clock_address.to_string(),
            liquid_pooler_address.to_string(),
        )?,
        // preset_holder_fields.to_instantiate2_msg(
        //     env.contract.address.to_string(),
        //     holder_salt,
        //     clock_address.to_string(),
        //     liquid_pooler_address.to_string(),
        //     party_a_router_address.to_string(),
        //     party_b_router_address.to_string(),
        // )?,
        preset_liquid_pooler_fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            liquid_pooler_salt,
            msg.pool_address,
            clock_address.to_string(),
            holder_address.to_string(),
        )?,
    ];

    if let Some(fields) = preset_forwarder_a_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            forwarder_a_salt,
            clock_address.to_string(),
            holder_address.to_string(),
        )?);
    }

    if let Some(fields) = preset_forwarder_b_fields {
        messages.push(fields.to_instantiate2_msg(
            env.contract.address.to_string(),
            forwarder_b_salt,
            clock_address.to_string(),
            holder_address.to_string(),
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
        QueryMsg::IbcForwarderAddress { party } => {
            let resp = if party == "party_a" {
                IBC_FORWARDER_A_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                IBC_FORWARDER_B_ADDR.may_load(deps.storage)?
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
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    match msg {
        MigrateMsg::MigrateContracts {
            clock,
            forwarder_a,
            forwarder_b,
            holder: _, // TODO: Holder
            liquid_pooler,
            splitter,
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

            if let Some(forwarder) = forwarder_a {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_FORWARDER_A_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_a_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: IBC_FORWARDER_A_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = forwarder_b {
                let msg: Binary = to_json_binary(&forwarder)?;
                let forwarder_fields = PRESET_FORWARDER_B_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_b_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: IBC_FORWARDER_B_ADDR.load(deps.storage)?.to_string(),
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

            // if let Some(holder) = holder {
            //     let msg: Binary = to_json_binary(&holder)?;
            //     let holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;
            //     resp = resp.add_attribute("holder_migrate", msg.to_base64());
            //     migrate_msgs.push(WasmMsg::Migrate {
            //         contract_addr: COVENANT_POL_HOLDER_ADDR.load(deps.storage)?.to_string(),
            //         new_code_id: holder_fields.code_id,
            //         msg,
            //     });
            // }

            Ok(resp.add_messages(migrate_msgs))
        }
    }
}
