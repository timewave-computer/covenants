#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CLOCK_CODE, COVENANT_CLOCK_ADDR, COVENANT_DEPOSITOR_ADDR, COVENANT_HOLDER_ADDR,
        COVENANT_LP_ADDR, COVENANT_LS_ADDR, DEPOSITOR_CODE, HOLDER_CODE, IBC_FEE, IBC_TIMEOUT,
        LP_CODE, LS_CODE, POOL_ADDRESS, PRESET_CLOCK_FIELDS, PRESET_DEPOSITOR_FIELDS,
        PRESET_HOLDER_FIELDS, PRESET_LP_FIELDS, PRESET_LS_FIELDS,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 60 * 24 * 7 * 2;

const CLOCK_REPLY_ID: u64 = 1u64;
const HOLDER_REPLY_ID: u64 = 2u64;
const LP_REPLY_ID: u64 = 3u64;
const LS_REPLY_ID: u64 = 4u64;
const DEPOSITOR_REPLY_ID: u64 = 5u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    LP_CODE.save(deps.storage, &msg.preset_lp_fields.lp_code)?;
    DEPOSITOR_CODE.save(deps.storage, &msg.preset_depositor_fields.depositor_code)?;
    LS_CODE.save(deps.storage, &msg.preset_ls_fields.ls_code)?;
    HOLDER_CODE.save(deps.storage, &msg.preset_holder_fields.holder_code)?;
    CLOCK_CODE.save(deps.storage, &msg.preset_clock_fields.clock_code)?;

    POOL_ADDRESS.save(deps.storage, &msg.pool_address)?;

    PRESET_CLOCK_FIELDS.save(deps.storage, &msg.preset_clock_fields)?;
    PRESET_LP_FIELDS.save(deps.storage, &msg.preset_lp_fields)?;
    PRESET_LS_FIELDS.save(deps.storage, &msg.preset_ls_fields)?;
    PRESET_DEPOSITOR_FIELDS.save(deps.storage, &msg.preset_depositor_fields)?;
    PRESET_HOLDER_FIELDS.save(deps.storage, &msg.preset_holder_fields)?;

    let ibc_timeout = if let Some(timeout) = msg.ibc_msg_transfer_timeout_timestamp {
        timeout
    } else {
        DEFAULT_TIMEOUT_SECONDS
    };
    IBC_TIMEOUT.save(deps.storage, &ibc_timeout)?;
    IBC_FEE.save(deps.storage, &msg.ibc_fee)?;

    let clock_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: msg.preset_clock_fields.clock_code,
        msg: to_binary(&msg.preset_clock_fields.clone().to_instantiate_msg())?,
        funds: vec![],
        label: msg.preset_clock_fields.label,
    });

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_success(
            clock_instantiate_tx,
            CLOCK_REPLY_ID,
        ))
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CLOCK_REPLY_ID => handle_clock_reply(deps, env, msg),
        HOLDER_REPLY_ID => handle_holder_reply(deps, env, msg),
        LP_REPLY_ID => handle_lp_reply(deps, env, msg),
        LS_REPLY_ID => handle_ls_reply(deps, env, msg),
        DEPOSITOR_REPLY_ID => handle_depositor_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
    }
}

pub fn handle_clock_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: clock reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // successful clock instantiation means we are ready to proceed with
            // remaining instantiations
            COVENANT_CLOCK_ADDR.save(
                deps.storage,
                &deps.api.addr_validate(&response.contract_address)?,
            )?;
            let pool_address = POOL_ADDRESS.load(deps.storage)?;

            let code_id = HOLDER_CODE.load(deps.storage)?;
            let preset_holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;

            let holder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(
                    &preset_holder_fields
                        .clone()
                        .to_instantiate_msg(pool_address),
                )?,
                funds: vec![],
                label: preset_holder_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_clock_reply")
                .add_submessage(SubMsg::reply_always(holder_instantiate_tx, HOLDER_REPLY_ID)))
        }
        Err(_err) => Err(ContractError::ContractInstantiationError {
            contract: "clock".to_string(),
        }),
    }
}

pub fn handle_holder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: holder reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_HOLDER_ADDR.save(
                deps.storage,
                &deps.api.addr_validate(&response.contract_address)?,
            )?;

            let pool_address = POOL_ADDRESS.load(deps.storage)?;
            let code_id = LP_CODE.load(deps.storage)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let preset_lp_fields = PRESET_LP_FIELDS.load(deps.storage)?;

            let instantiate_msg = preset_lp_fields.clone().to_instantiate_msg(
                clock_addr.to_string(),
                response.contract_address,
                pool_address,
            );

            let lp_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: preset_lp_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_holder_reply")
                .add_submessage(SubMsg::reply_always(lp_instantiate_tx, LP_REPLY_ID)))
        }
        Err(_err) => Err(ContractError::ContractInstantiationError {
            contract: "holder".to_string(),
        }),
    }
}

pub fn handle_lp_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: lp reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // store the lp address to fill other InstantiateMsg
            COVENANT_LP_ADDR.save(
                deps.storage,
                &deps.api.addr_validate(&response.contract_address)?,
            )?;

            // load missing params
            let clock_address = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let code_id = LS_CODE.load(deps.storage)?;
            let preset_ls_fields = PRESET_LS_FIELDS.load(deps.storage)?;
            let ibc_timeout = IBC_TIMEOUT.load(deps.storage)?;
            let ibc_fee = IBC_FEE.load(deps.storage)?;

            let instantiate_msg = preset_ls_fields.clone().to_instantiate_msg(
                clock_address.to_string(),
                response.contract_address,
                ibc_timeout,
                ibc_fee,
            );

            let ls_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: preset_ls_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_lp_reply")
                .add_submessage(SubMsg::reply_always(ls_instantiate_tx, LS_REPLY_ID)))
        }
        Err(_err) => Err(ContractError::ContractInstantiationError {
            contract: "lp".to_string(),
        }),
    }
}

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_LS_ADDR.save(
                deps.storage,
                &deps.api.addr_validate(&response.contract_address)?,
            )?;

            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let lp_addr = COVENANT_LP_ADDR.load(deps.storage)?;
            let code_id = DEPOSITOR_CODE.load(deps.storage)?;
            let preset_depositor_fields = PRESET_DEPOSITOR_FIELDS.load(deps.storage)?;
            let ibc_timeout = IBC_TIMEOUT.load(deps.storage)?;
            let ibc_fee = IBC_FEE.load(deps.storage)?;

            let instantiate_msg = preset_depositor_fields.clone().to_instantiate_msg(
                "to be queried".to_string(),
                clock_addr.to_string(),
                response.contract_address,
                lp_addr.to_string(),
                ibc_timeout,
                ibc_fee,
            );

            let depositor_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: preset_depositor_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_holder_reply")
                .add_submessage(SubMsg::reply_always(
                    depositor_instantiate_tx,
                    DEPOSITOR_REPLY_ID,
                )))
        }
        Err(_err) => Err(ContractError::ContractInstantiationError {
            contract: "ls".to_string(),
        }),
    }
}

pub fn handle_depositor_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: depositor reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_DEPOSITOR_ADDR.save(
                deps.storage,
                &deps.api.addr_validate(&response.contract_address)?,
            )?;

            // this is the last reply, we can now whitelist all contracts on the clock
            // and it will automatically enqueue them.
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let clock_code_id = CLOCK_CODE.load(deps.storage)?;
            let lp_addr = COVENANT_LP_ADDR.load(deps.storage)?;
            let ls_addr = COVENANT_LS_ADDR.load(deps.storage)?;

            let migrate_msg = WasmMsg::Migrate {
                contract_addr: clock_addr.to_string(),
                new_code_id: clock_code_id,
                msg: to_binary(&covenant_clock::msg::MigrateMsg:: {
                    contracts: vec![lp_addr.to_string(), ls_addr.to_string()],
                })?,
            };

            Ok(Response::default().add_attribute("method", "handle_depositor_reply"))
        }
        Err(_err) => Err(ContractError::ContractInstantiationError {
            contract: "depositor".to_string(),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DepositorAddress {} => {
            Ok(to_binary(&COVENANT_DEPOSITOR_ADDR.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_binary(&COVENANT_CLOCK_ADDR.may_load(deps.storage)?)?),
        QueryMsg::LpAddress {} => Ok(to_binary(&COVENANT_LP_ADDR.may_load(deps.storage)?)?),
        QueryMsg::LsAddress {} => Ok(to_binary(&COVENANT_LS_ADDR.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&COVENANT_HOLDER_ADDR.may_load(deps.storage)?)?),
        QueryMsg::PoolAddress {} => Ok(to_binary(&POOL_ADDRESS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::MigrateContracts {
            clock,
            depositor,
            lp,
            ls,
            holder,
        } => {
            let mut migrate_msgs = vec![];

            if let Some(clock) = clock {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: CLOCK_CODE.load(deps.storage)?,
                    msg: to_binary(&clock)?,
                })
            }

            if let Some(depositor) = depositor {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_DEPOSITOR_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: DEPOSITOR_CODE.load(deps.storage)?,
                    msg: to_binary(&depositor)?,
                })
            }

            if let Some(lp) = lp {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_LP_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: LP_CODE.load(deps.storage)?,
                    msg: to_binary(&lp)?,
                })
            }

            if let Some(ls) = ls {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_LS_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: LS_CODE.load(deps.storage)?,
                    msg: to_binary(&ls)?,
                })
            }

            if let Some(holder) = holder {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_HOLDER_ADDR.load(deps.storage)?.to_string(),
                    new_code_id: HOLDER_CODE.load(deps.storage)?,
                    msg: to_binary(&holder)?,
                })
            }

            Ok(Response::default()
                .add_attribute("method", "update_config")
                .add_messages(migrate_msgs))
        }
    }
}
