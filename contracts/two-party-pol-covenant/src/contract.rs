#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, SubMsg, Uint128, WasmMsg,
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
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_POL_HOLDER_ADDR, LIQUID_POOLER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_ROUTER_ADDR, PRESET_CLOCK_FIELDS, PRESET_HOLDER_FIELDS,
        PRESET_LIQUID_POOLER_FIELDS, PRESET_PARTY_A_FORWARDER_FIELDS, PRESET_PARTY_A_ROUTER_FIELDS,
        PRESET_PARTY_B_FORWARDER_FIELDS, PRESET_PARTY_B_ROUTER_FIELDS,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-two-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_REPLY_ID: u64 = 1u64;
pub const HOLDER_REPLY_ID: u64 = 2u64;
pub const PARTY_A_FORWARDER_REPLY_ID: u64 = 3u64;
pub const PARTY_B_FORWARDER_REPLY_ID: u64 = 4u64;
pub const LP_REPLY_ID: u64 = 5u64;
pub const PARTY_A_ROUTER_REPLY_ID: u64 = 6u64;
pub const PARTY_B_ROUTER_REPLY_ID: u64 = 7u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: vec![],
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };
    let preset_holder_fields = PresetTwoPartyPolHolderFields {
        lockup_config: msg.lockup_config,
        pool_address: msg.pool_address,
        ragequit_config: msg.ragequit_config.unwrap_or(RagequitConfig::Disabled),
        deposit_deadline: msg.deposit_deadline,
        party_a: PresetPolParty {
            contribution: Coin {
                denom: msg.party_a_config.ibc_denom.to_string(),
                amount: msg.party_a_config.contribution.amount,
            },
            controller_addr: msg.party_a_config.controller_addr.to_string(),
            host_addr: msg.party_a_config.host_addr,
            allocation: Decimal::from_ratio(msg.party_a_share, Uint128::new(100)),
        },
        party_b: PresetPolParty {
            contribution: Coin {
                denom: msg.party_b_config.ibc_denom.to_string(),
                amount: msg.party_b_config.contribution.amount,
            },
            controller_addr: msg.party_b_config.controller_addr.to_string(),
            host_addr: msg.party_b_config.host_addr,
            allocation: Decimal::from_ratio(msg.party_b_share, Uint128::new(100)),
        },
        code_id: msg.contract_codes.holder_code,
        label: format!("{}-holder", msg.label),
        splits: msg.splits,
        fallback_split: msg.fallback_split,
        covenant_type: msg.covenant_type,
    };
    let preset_party_a_forwarder_fields = PresetIbcForwarderFields {
        remote_chain_connection_id: msg.party_a_config.party_chain_connection_id,
        remote_chain_channel_id: msg.party_a_config.party_to_host_chain_channel_id,
        denom: msg.party_a_config.contribution.denom.to_string(),
        amount: msg.party_a_config.contribution.amount,
        label: format!("{}_party_a_ibc_forwarder", msg.label),
        code_id: msg.contract_codes.ibc_forwarder_code,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
    };
    let preset_party_b_forwarder_fields = PresetIbcForwarderFields {
        remote_chain_connection_id: msg.party_b_config.party_chain_connection_id,
        remote_chain_channel_id: msg.party_b_config.party_to_host_chain_channel_id,
        denom: msg.party_b_config.contribution.denom.to_string(),
        amount: msg.party_b_config.contribution.amount,
        label: format!("{}_party_b_ibc_forwarder", msg.label),
        code_id: msg.contract_codes.ibc_forwarder_code,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
    };

    let preset_party_a_router_fields = PresetInterchainRouterFields {
        destination_chain_channel_id: msg.party_a_config.host_to_party_chain_channel_id,
        destination_receiver_addr: msg.party_a_config.controller_addr,
        ibc_transfer_timeout: msg.party_a_config.ibc_transfer_timeout,
        label: format!("{}_party_a_interchain_router", msg.label),
        code_id: msg.contract_codes.router_code,
    };
    let preset_party_b_router_fields = PresetInterchainRouterFields {
        destination_chain_channel_id: msg.party_b_config.host_to_party_chain_channel_id,
        destination_receiver_addr: msg.party_b_config.controller_addr,
        ibc_transfer_timeout: msg.party_b_config.ibc_transfer_timeout,
        label: format!("{}_party_b_interchain_router", msg.label),
        code_id: msg.contract_codes.router_code,
    };

    let preset_liquid_pooler_fields = PresetAstroLiquidPoolerFields {
        slippage_tolerance: None,
        assets: AssetData {
            asset_a_denom: msg.party_a_config.ibc_denom,
            asset_b_denom: msg.party_b_config.ibc_denom,
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

    PRESET_CLOCK_FIELDS.save(deps.storage, &preset_clock_fields)?;
    PRESET_HOLDER_FIELDS.save(deps.storage, &preset_holder_fields)?;
    PRESET_PARTY_A_FORWARDER_FIELDS.save(deps.storage, &preset_party_a_forwarder_fields)?;
    PRESET_PARTY_B_FORWARDER_FIELDS.save(deps.storage, &preset_party_b_forwarder_fields)?;
    PRESET_PARTY_A_ROUTER_FIELDS.save(deps.storage, &preset_party_a_router_fields)?;
    PRESET_PARTY_B_ROUTER_FIELDS.save(deps.storage, &preset_party_b_router_fields)?;
    PRESET_LIQUID_POOLER_FIELDS.save(deps.storage, &preset_liquid_pooler_fields)?;

    // we start the module instantiation chain with the clock
    let clock_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: preset_clock_fields.code_id,
        msg: to_binary(&preset_clock_fields.to_instantiate_msg())?,
        funds: vec![],
        label: preset_clock_fields.label,
    });

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_submessage(SubMsg::reply_on_success(
            clock_instantiate_tx,
            CLOCK_REPLY_ID,
        )))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CLOCK_REPLY_ID => handle_clock_reply(deps, env, msg),
        PARTY_A_ROUTER_REPLY_ID => handle_party_a_interchain_router_reply(deps, env, msg),
        PARTY_B_ROUTER_REPLY_ID => handle_party_b_interchain_router_reply(deps, env, msg),
        HOLDER_REPLY_ID => handle_holder_reply(deps, env, msg),
        PARTY_A_FORWARDER_REPLY_ID => handle_party_a_ibc_forwarder_reply(deps, env, msg),
        PARTY_B_FORWARDER_REPLY_ID => handle_party_b_ibc_forwarder_reply(deps, env, msg),
        LP_REPLY_ID => handle_liquid_pooler_reply_id(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
    }
}

pub fn handle_clock_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: clock reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the clock address
            let clock_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;

            let party_a_router_preset_fields = PRESET_PARTY_A_ROUTER_FIELDS.load(deps.storage)?;

            let party_a_router_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: party_a_router_preset_fields.code_id,
                msg: to_binary(
                    &party_a_router_preset_fields.to_instantiate_msg(clock_addr.to_string()),
                )?,
                funds: vec![],
                label: party_a_router_preset_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_clock_reply")
                .add_attribute("clock_addr", clock_addr)
                .add_attribute(
                    "router_code_id",
                    party_a_router_preset_fields.code_id.to_string(),
                )
                .add_attribute(
                    "party_a_addr",
                    party_a_router_preset_fields.destination_receiver_addr,
                )
                .add_submessage(SubMsg::reply_always(
                    party_a_router_instantiate_tx,
                    PARTY_A_ROUTER_REPLY_ID,
                )))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "clock".to_string(),
            err,
        }),
    }
}

pub fn handle_party_a_interchain_router_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party A interchain router reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated router address
            let router_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_A_ROUTER_ADDR.save(deps.storage, &router_addr)?;

            // load the fields relevant to router instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let party_b_router_preset_fields = PRESET_PARTY_B_ROUTER_FIELDS.load(deps.storage)?;

            let party_b_router_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: party_b_router_preset_fields.code_id,
                msg: to_binary(
                    &party_b_router_preset_fields.to_instantiate_msg(clock_addr.to_string()),
                )?,
                funds: vec![],
                label: party_b_router_preset_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_a_interchain_router_reply")
                .add_attribute("party_a_interchain_router_addr", router_addr)
                .add_submessage(SubMsg::reply_always(
                    party_b_router_instantiate_tx,
                    PARTY_B_ROUTER_REPLY_ID,
                )))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party a router".to_string(),
            err,
        }),
    }
}

pub fn handle_party_b_interchain_router_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party B interchain router reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated router address
            let router_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_B_ROUTER_ADDR.save(deps.storage, &router_addr)?;

            let clock_address = COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string();
            let pool_address = PRESET_HOLDER_FIELDS.load(deps.storage)?.pool_address;
            let preset_liquid_pooler_fields = PRESET_LIQUID_POOLER_FIELDS.load(deps.storage)?;

            let instantiate_msg =
                preset_liquid_pooler_fields.to_instantiate_msg(pool_address, clock_address);

            let liquid_pooler_inst_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: preset_liquid_pooler_fields.code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: preset_liquid_pooler_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_b_interchain_router_reply")
                .add_attribute("party_b_interchain_router_addr", router_addr)
                .add_submessage(SubMsg::reply_always(liquid_pooler_inst_tx, LP_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party b router".to_string(),
            err,
        }),
    }
}

pub fn handle_liquid_pooler_reply_id(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: liquid pooler reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated liquid pooler address
            let liquid_pooler = deps.api.addr_validate(&response.contract_address)?;
            LIQUID_POOLER_ADDR.save(deps.storage, &liquid_pooler)?;

            let party_b_router = PARTY_B_ROUTER_ADDR.load(deps.storage)?;
            let preset_holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let party_a_router = PARTY_A_ROUTER_ADDR.load(deps.storage)?;

            let instantiate_msg = preset_holder_fields.clone().to_instantiate_msg(
                clock_addr.to_string(),
                liquid_pooler.to_string(),
                party_a_router.as_str(),
                party_b_router.as_str(),
            )?;

            let holder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: preset_holder_fields.code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: preset_holder_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_liquid_pooler_reply")
                .add_attribute("liquid_pooler_addr", liquid_pooler)
                .add_attribute(
                    "holder_instantiate_tx",
                    to_binary(&holder_instantiate_tx)?.to_string(),
                )
                .add_submessage(SubMsg::reply_always(holder_instantiate_tx, HOLDER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "liquid pooler".to_string(),
            err,
        }),
    }
}

pub fn handle_holder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: holder reply");
    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated holder address
            let holder_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_POL_HOLDER_ADDR.save(deps.storage, &holder_addr)?;

            // load the fields relevant to router instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let preset_party_a_ibc_forwarder =
                PRESET_PARTY_A_FORWARDER_FIELDS.load(deps.storage)?;

            let party_a_ibc_forwarder_inst_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: preset_party_a_ibc_forwarder.code_id,
                msg: to_binary(
                    &preset_party_a_ibc_forwarder
                        .to_instantiate_msg(clock_addr.to_string(), holder_addr.to_string()),
                )?,
                funds: vec![],
                label: preset_party_a_ibc_forwarder.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_holder_reply")
                .add_attribute("holder_addr", holder_addr)
                .add_submessage(SubMsg::reply_always(
                    party_a_ibc_forwarder_inst_tx,
                    PARTY_A_FORWARDER_REPLY_ID,
                )))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "holder".to_string(),
            err,
        }),
    }
}

pub fn handle_party_a_ibc_forwarder_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party A ibc forwarder reply");
    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated forwarder address
            let forwarder_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &forwarder_addr)?;
            let holder = COVENANT_POL_HOLDER_ADDR.load(deps.storage)?;

            // load the fields relevant to router instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let preset_party_b_ibc_forwarder =
                PRESET_PARTY_B_FORWARDER_FIELDS.load(deps.storage)?;

            let party_b_ibc_forwarder_inst_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: preset_party_b_ibc_forwarder.code_id,
                msg: to_binary(
                    &preset_party_b_ibc_forwarder
                        .to_instantiate_msg(clock_addr.to_string(), holder.to_string()),
                )?,
                funds: vec![],
                label: preset_party_b_ibc_forwarder.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_a_ibc_forwarder_reply")
                .add_attribute("PARTY_A_IBC_FORWARDER_ADDR", forwarder_addr)
                .add_submessage(SubMsg::reply_always(
                    party_b_ibc_forwarder_inst_tx,
                    PARTY_B_FORWARDER_REPLY_ID,
                )))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "PARTY_A_IBC_FORWARDER_ADDR".to_string(),
            err,
        }),
    }
}

pub fn handle_party_b_ibc_forwarder_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party B ibc forwarder reply");
    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the party b ibc forwarder address
            let party_b_ibc_forwarder_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_ibc_forwarder_addr)?;

            let party_a_forwarder = PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let preset_clock_fields = PRESET_CLOCK_FIELDS.load(deps.storage)?;
            let holder = COVENANT_POL_HOLDER_ADDR.load(deps.storage)?;
            let party_a_router = PARTY_A_ROUTER_ADDR.load(deps.storage)?;
            let party_b_router = PARTY_B_ROUTER_ADDR.load(deps.storage)?;
            let liquid_pooler = LIQUID_POOLER_ADDR.load(deps.storage)?;

            let lp_fields = PRESET_LIQUID_POOLER_FIELDS.load(deps.storage)?;

            let update_liquid_pooler_holder_addr = WasmMsg::Migrate {
                contract_addr: liquid_pooler.to_string(),
                new_code_id: lp_fields.code_id,
                msg: to_binary(
                    &covenant_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
                        clock_addr: None,
                        holder_address: Some(holder.to_string()),
                        lp_config: None,
                    },
                )?,
            };

            let update_clock_whitelist_msg = WasmMsg::Migrate {
                contract_addr: clock_addr.to_string(),
                new_code_id: preset_clock_fields.code_id,
                msg: to_binary(&covenant_clock::msg::MigrateMsg::ManageWhitelist {
                    add: Some(vec![
                        party_a_forwarder.to_string(),
                        party_b_ibc_forwarder_addr.to_string(),
                        holder.to_string(),
                        party_a_router.to_string(),
                        party_b_router.to_string(),
                        liquid_pooler.to_string(),
                    ]),
                    remove: None,
                })?,
            };

            Ok(Response::default()
                .add_attribute("method", "handle_party_b_ibc_forwarder_reply")
                .add_attribute("party_b_ibc_forwarder_addr", party_b_ibc_forwarder_addr)
                .add_message(update_clock_whitelist_msg)
                .add_message(update_liquid_pooler_holder_addr))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party_b ibc forwarder".to_string(),
            err,
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&COVENANT_CLOCK_ADDR.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(
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
            Ok(to_binary(&resp)?)
        }
        QueryMsg::InterchainRouterAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_ROUTER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_ROUTER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_binary(&resp)?)
        }
        QueryMsg::LiquidPoolerAddress {} => {
            Ok(to_binary(&LIQUID_POOLER_ADDR.may_load(deps.storage)?)?)
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
                let msg = to_binary(&clock)?;
                let clock_fields = PRESET_CLOCK_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("clock_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: clock_fields.code_id,
                    msg,
                });
            }

            if let Some(router) = party_a_router {
                let msg: Binary = to_binary(&router)?;
                let router_fields = PRESET_PARTY_A_ROUTER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_a_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: router_fields.code_id,
                    msg,
                });
            }

            if let Some(router) = party_b_router {
                let msg: Binary = to_binary(&router)?;
                let router_fields = PRESET_PARTY_B_ROUTER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_b_router_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_ROUTER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: router_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = party_a_forwarder {
                let msg: Binary = to_binary(&forwarder)?;
                let forwarder_fields = PRESET_PARTY_A_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_a_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(forwarder) = party_b_forwarder {
                let msg: Binary = to_binary(&forwarder)?;
                let forwarder_fields = PRESET_PARTY_B_FORWARDER_FIELDS.load(deps.storage)?;
                resp = resp.add_attribute("party_b_forwarder_migrate", msg.to_base64());
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: PARTY_B_IBC_FORWARDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: forwarder_fields.code_id,
                    msg,
                });
            }

            if let Some(holder) = holder {
                let msg: Binary = to_binary(&holder)?;
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
