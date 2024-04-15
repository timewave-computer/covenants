use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use covenant_utils::split::SplitConfig;
use covenant_utils::{instantiate2_helper::get_instantiate2_salt_and_address, DestinationConfig};
use cw2::set_contract_version;
use valence_ibc_forwarder::msg::InstantiateMsg as IbcForwarderInstantiateMsg;
use valence_interchain_router::msg::InstantiateMsg as RouterInstantiateMsg;
use valence_remote_chain_splitter::msg::InstantiateMsg as SplitterInstantiateMsg;
use valence_single_party_pol_holder::msg::InstantiateMsg as HolderInstantiateMsg;
use valence_stride_liquid_staker::msg::InstantiateMsg as LiquidStakerInstantiateMsg;

use crate::msg::LiquidPoolerMigrateMsg;
use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CONTRACT_CODES, COVENANT_CLOCK_ADDR, HOLDER_ADDR, LIQUID_POOLER_ADDR, LIQUID_STAKER_ADDR,
        LP_FORWARDER_ADDR, LS_FORWARDER_ADDR, ROUTER_ADDR, SPLITTER_ADDR,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const CLOCK_SALT: &[u8] = b"clock";
pub(crate) const HOLDER_SALT: &[u8] = b"pol_holder";
pub(crate) const REMOTE_CHAIN_SPLITTER_SALT: &[u8] = b"remote_chain_splitter";
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
    let clock_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        CLOCK_SALT,
        &creator_address,
        msg.contract_codes.clock_code,
    )?;
    let splitter_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        REMOTE_CHAIN_SPLITTER_SALT,
        &creator_address,
        msg.contract_codes.remote_chain_splitter_code,
    )?;
    let ls_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LS_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;
    let lp_forwarder_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LP_FORWARDER_SALT,
        &creator_address,
        msg.contract_codes.ibc_forwarder_code,
    )?;
    let liquid_staker_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_STAKER_SALT,
        &creator_address,
        msg.contract_codes.liquid_staker_code,
    )?;
    let liquid_pooler_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        LIQUID_POOLER_SALT,
        &creator_address,
        msg.contract_codes.liquid_pooler_code,
    )?;
    let holder_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        HOLDER_SALT,
        &creator_address,
        msg.contract_codes.holder_code,
    )?;
    let router_instantiate2_config = get_instantiate2_salt_and_address(
        deps.as_ref(),
        ROUTER_SALT,
        &creator_address,
        msg.contract_codes.interchain_router_code,
    )?;

    let mut clock_whitelist = Vec::with_capacity(7);
    clock_whitelist.push(splitter_instantiate2_config.addr.to_string());
    clock_whitelist.push(liquid_pooler_instantiate2_config.addr.to_string());
    clock_whitelist.push(liquid_staker_instantiate2_config.addr.to_string());
    clock_whitelist.push(holder_instantiate2_config.addr.to_string());
    clock_whitelist.push(router_instantiate2_config.addr.to_string());

    let mut denoms: BTreeSet<String> = BTreeSet::new();
    denoms.insert(msg.ls_info.ls_denom_on_neutron.to_string());
    denoms.insert(msg.covenant_party_config.native_denom.to_string());

    let router_instantiate2_msg = RouterInstantiateMsg {
        clock_address: clock_instantiate2_config.addr.to_string(),
        destination_config: DestinationConfig {
            local_to_destination_chain_channel_id: msg
                .covenant_party_config
                .host_to_party_chain_channel_id
                .to_string(),
            destination_receiver_addr: msg.covenant_party_config.party_receiver_addr.to_string(),
            ibc_transfer_timeout: msg.covenant_party_config.ibc_transfer_timeout,
            denom_to_pfm_map: msg.covenant_party_config.denom_to_pfm_map,
        },
        denoms,
    }
    .to_instantiate2_msg(
        &router_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_interchain_router", msg.label),
    )?;

    let holder_instantiate2_msg = HolderInstantiateMsg {
        withdrawer: msg.covenant_party_config.addr.to_string(),
        withdraw_to: router_instantiate2_config.addr.to_string(),
        emergency_committee_addr: msg.emergency_committee.clone(),
        lockup_period: msg.lockup_period,
        pooler_address: liquid_pooler_instantiate2_config.addr.to_string(),
    }
    .to_instantiate2_msg(
        &holder_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_holder", msg.label),
    )?;

    let liquid_staker_instantiate2_msg = LiquidStakerInstantiateMsg {
        ls_denom: msg.ls_info.ls_denom.to_string(),
        stride_neutron_ibc_transfer_channel_id: msg
            .ls_info
            .ls_chain_to_neutron_channel_id
            .to_string(),
        neutron_stride_ibc_connection_id: msg.ls_info.ls_neutron_connection_id.to_string(),
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        clock_address: clock_instantiate2_config.addr.to_string(),
        next_contract: liquid_pooler_instantiate2_config.addr.to_string(),
    }
    .to_instantiate2_msg(
        &liquid_staker_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_liquid_staker", msg.label),
    )?;

    let liquid_pooler_instantiate2_msg = msg.liquid_pooler_config.to_instantiate2_msg(
        &liquid_pooler_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_liquid_pooler", msg.label),
        clock_instantiate2_config.addr.to_string(),
        holder_instantiate2_config.addr.to_string(),
        msg.pool_price_config,
    )?;

    let mut split_config_map: BTreeMap<String, Decimal> = BTreeMap::new();
    split_config_map.insert(
        ls_forwarder_instantiate2_config.addr.to_string(),
        msg.remote_chain_splitter_config.ls_share,
    );
    split_config_map.insert(
        lp_forwarder_instantiate2_config.addr.to_string(),
        msg.remote_chain_splitter_config.native_share,
    );

    let mut splits: BTreeMap<String, SplitConfig> = BTreeMap::new();
    splits.insert(
        msg.remote_chain_splitter_config.denom.to_string(),
        SplitConfig {
            receivers: split_config_map,
        },
    );

    let splitter_instantiate2_msg = SplitterInstantiateMsg {
        clock_address: clock_instantiate2_config.addr.to_string(),
        remote_chain_channel_id: msg.remote_chain_splitter_config.channel_id,
        remote_chain_connection_id: msg.remote_chain_splitter_config.connection_id,
        denom: msg.remote_chain_splitter_config.denom.to_string(),
        amount: msg.remote_chain_splitter_config.amount,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        splits,
        fallback_address: msg.remote_chain_splitter_config.fallback_address,
    }
    .to_instantiate2_msg(
        &splitter_instantiate2_config,
        env.contract.address.to_string(),
        format!("{}_remote_chain_splitter", msg.label),
    )?;

    let mut messages = vec![
        liquid_staker_instantiate2_msg,
        holder_instantiate2_msg,
        liquid_pooler_instantiate2_msg,
        splitter_instantiate2_msg,
        router_instantiate2_msg,
    ];

    if let CovenantPartyConfig::Interchain(config) = msg.ls_forwarder_config {
        LS_FORWARDER_ADDR.save(deps.storage, &ls_forwarder_instantiate2_config.addr)?;
        clock_whitelist.insert(0, ls_forwarder_instantiate2_config.addr.to_string());
        let instantiate_msg = IbcForwarderInstantiateMsg {
            clock_address: clock_instantiate2_config.addr.to_string(),
            next_contract: liquid_staker_instantiate2_config.addr.to_string(),
            remote_chain_connection_id: config.party_chain_connection_id,
            remote_chain_channel_id: config.party_to_host_chain_channel_id,
            denom: config.remote_chain_denom,
            amount: config.contribution.amount,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            ica_timeout: msg.timeouts.ica_timeout,
            fallback_address: config.fallback_address,
        };
        messages.push(instantiate_msg.to_instantiate2_msg(
            &ls_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_ls_ibc_forwarder", msg.label),
        )?);
    }

    if let CovenantPartyConfig::Interchain(config) = msg.lp_forwarder_config {
        LP_FORWARDER_ADDR.save(deps.storage, &lp_forwarder_instantiate2_config.addr)?;
        clock_whitelist.insert(0, lp_forwarder_instantiate2_config.addr.to_string());
        let instantiate_msg = IbcForwarderInstantiateMsg {
            clock_address: clock_instantiate2_config.addr.to_string(),
            next_contract: liquid_pooler_instantiate2_config.addr.to_string(),
            remote_chain_connection_id: config.party_chain_connection_id,
            remote_chain_channel_id: config.party_to_host_chain_channel_id,
            denom: config.remote_chain_denom,
            amount: config.contribution.amount,
            ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
            ica_timeout: msg.timeouts.ica_timeout,
            fallback_address: config.fallback_address,
        };
        messages.push(instantiate_msg.to_instantiate2_msg(
            &lp_forwarder_instantiate2_config,
            env.contract.address.to_string(),
            format!("{}_lp_ibc_forwarder", msg.label),
        )?);
    };

    let clock_instantiate2_msg = valence_clock::msg::InstantiateMsg {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: clock_whitelist,
    }
    .to_instantiate2_msg(
        clock_instantiate2_config.code,
        clock_instantiate2_config.salt,
        env.contract.address.to_string(),
        format!("{}-clock", msg.label),
    )?;
    messages.insert(0, clock_instantiate2_msg);

    HOLDER_ADDR.save(deps.storage, &holder_instantiate2_config.addr)?;
    LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler_instantiate2_config.addr)?;
    LIQUID_STAKER_ADDR.save(deps.storage, &liquid_staker_instantiate2_config.addr)?;
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_instantiate2_config.addr)?;
    SPLITTER_ADDR.save(deps.storage, &splitter_instantiate2_config.addr)?;
    ROUTER_ADDR.save(deps.storage, &router_instantiate2_config.addr)?;
    LS_FORWARDER_ADDR.save(deps.storage, &ls_forwarder_instantiate2_config.addr)?;
    LP_FORWARDER_ADDR.save(deps.storage, &lp_forwarder_instantiate2_config.addr)?;
    CONTRACT_CODES.save(deps.storage, &msg.contract_codes)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("clock_addr", clock_instantiate2_config.addr)
        .add_attribute("ls_forwarder_addr", ls_forwarder_instantiate2_config.addr)
        .add_attribute("lp_forwarder_addr", lp_forwarder_instantiate2_config.addr)
        .add_attribute("holder_addr", holder_instantiate2_config.addr)
        .add_attribute("splitter_addr", splitter_instantiate2_config.addr)
        .add_attribute("liquid_staker_addr", liquid_staker_instantiate2_config.addr)
        .add_attribute("liquid_pooler_addr", liquid_pooler_instantiate2_config.addr)
        .add_attribute("router_addr", router_instantiate2_config.addr)
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
                return Err(cosmwasm_std::StdError::not_found(
                    "unknown type".to_string(),
                ));
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
                &covenant_utils::neutron::CovenantQueryMsg::DepositAddress {},
            )?;

            Ok(to_json_binary(&ica)?)
        }
        QueryMsg::ContractCodes {} => Ok(to_json_binary(&CONTRACT_CODES.load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::MigrateContracts {
            codes,
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

            if let Some(liquid_pooler_migrate_msg) = liquid_pooler {
                let msg: Binary = match liquid_pooler_migrate_msg {
                    LiquidPoolerMigrateMsg::Astroport(msg) => to_json_binary(&msg)?,
                    LiquidPoolerMigrateMsg::Osmosis(msg) => to_json_binary(&msg)?,
                };
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
                    new_code_id: contract_codes.remote_chain_splitter_code,
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
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
