#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CLOCK_CODE, CLOCK_INSTANTIATION_DATA, COVENANT_CLOCK_ADDR, COVENANT_DEPOSITOR_ADDR,
        COVENANT_HOLDER_ADDR, COVENANT_LP_ADDR, COVENANT_LS_ADDR, DEPOSITOR_CODE,
        DEPOSITOR_INSTANTIATION_DATA, HOLDER_CODE, HOLDER_INSTANTIATION_DATA, LP_CODE,
        LP_INSTANTIATION_DATA, LS_CODE, LS_INSTANTIATION_DATA,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    LP_CODE.save(deps.storage, &msg.lp_code)?;
    DEPOSITOR_CODE.save(deps.storage, &msg.depositor_code)?;
    LS_CODE.save(deps.storage, &msg.ls_code)?;
    HOLDER_CODE.save(deps.storage, &msg.holder_code)?;
    CLOCK_CODE.save(deps.storage, &msg.clock_code)?;

    CLOCK_INSTANTIATION_DATA.save(deps.storage, &msg.clock_instantiate.clone())?;
    LP_INSTANTIATION_DATA.save(deps.storage, &msg.lp_instantiate)?;
    LS_INSTANTIATION_DATA.save(deps.storage, &msg.ls_instantiate)?;
    DEPOSITOR_INSTANTIATION_DATA.save(deps.storage, &msg.depositor_instantiate)?;
    HOLDER_INSTANTIATION_DATA.save(deps.storage, &msg.holder_instantiate)?;

    let clock_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: msg.clock_code,
        msg: to_binary(&msg.clock_instantiate)?,
        funds: vec![],
        label: "covenant-clock".to_string(),
    });

    // instantiate clock first
    Ok(Response::default().add_submessage(SubMsg::reply_on_success(
        clock_instantiate_tx,
        CLOCK_REPLY_ID,
    )))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DepositorAddress {} => {
            Ok(to_binary(&COVENANT_DEPOSITOR_ADDR.may_load(deps.storage)?)?)
        }
        QueryMsg::ClockAddress {} => Ok(to_binary(&COVENANT_CLOCK_ADDR.may_load(deps.storage)?)?),
        QueryMsg::LpAddress {} => Ok(to_binary(&COVENANT_LP_ADDR.may_load(deps.storage)?)?),
        QueryMsg::LsAddress {} => Ok(to_binary(&COVENANT_LS_ADDR.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&COVENANT_HOLDER_ADDR.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock,
            depositer,
            lp,
            ls,
            holder,
        } => {
            let mut migrate_msgs = vec![];

            if let Some(clock) = clock {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_CLOCK_ADDR.load(deps.storage)?,
                    new_code_id: CLOCK_CODE.load(deps.storage)?,
                    msg: to_binary(&clock)?,
                })
            }

            if let Some(depositer) = depositer {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_DEPOSITOR_ADDR.load(deps.storage)?,
                    new_code_id: DEPOSITOR_CODE.load(deps.storage)?,
                    msg: to_binary(&depositer)?,
                })
            }

            if let Some(lp) = lp {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_LP_ADDR.load(deps.storage)?,
                    new_code_id: LP_CODE.load(deps.storage)?,
                    msg: to_binary(&lp)?,
                })
            }

            if let Some(ls) = ls {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_LS_ADDR.load(deps.storage)?,
                    new_code_id: LS_CODE.load(deps.storage)?,
                    msg: to_binary(&ls)?,
                })
            }

            if let Some(holder) = holder {
                migrate_msgs.push(WasmMsg::Migrate {
                    contract_addr: COVENANT_HOLDER_ADDR.load(deps.storage)?,
                    new_code_id: HOLDER_CODE.load(deps.storage)?,
                    msg: to_binary(&holder)?,
                })
            }

            Ok(Response::default().add_messages(migrate_msgs))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CLOCK_REPLY_ID => handle_clock_reply(deps, env, msg),
        HOLDER_REPLY_ID => handle_holder_reply(deps, env, msg),
        LP_REPLY_ID => handle_lp_reply(deps, env, msg),
        LS_REPLY_ID => handle_ls_reply(deps, env, msg),
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
            COVENANT_CLOCK_ADDR.save(deps.storage, &response.contract_address)?;

            let holder_code = HOLDER_CODE.load(deps.storage)?;
            let holder_data = HOLDER_INSTANTIATION_DATA.load(deps.storage)?;

            let holder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: holder_code,
                msg: to_binary(&holder_data)?,
                funds: vec![],
                label: "covenant-holder".to_string(),
            });

            Ok(Response::default().add_submessage(SubMsg::reply_on_success(
                holder_instantiate_tx,
                HOLDER_REPLY_ID,
            )))
        }
        Err(err) => Err(ContractError::Std(StdError::GenericErr {
            msg: err.to_string(),
        })),
    }
}

pub fn handle_holder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: holder reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_HOLDER_ADDR.save(deps.storage, &response.contract_address)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;

            let mut lp_data = LP_INSTANTIATION_DATA.load(deps.storage)?;
            lp_data.clock_address = clock_addr;
            lp_data.holder_address = response.contract_address;

            let lp_code = LP_CODE.load(deps.storage)?;
            let lp_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: lp_code,
                msg: to_binary(&lp_data)?,
                funds: vec![],
                label: "covenant-lp".to_string(),
            });

            Ok(Response::default()
                .add_submessage(SubMsg::reply_on_success(lp_instantiate_tx, LP_REPLY_ID)))
        }
        Err(err) => Err(ContractError::Std(StdError::GenericErr {
            msg: err.to_string(),
        })),
    }
}

pub fn handle_lp_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: lp reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_LP_ADDR.save(deps.storage, &response.contract_address)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;

            let ls_code = LS_CODE.load(deps.storage)?;
            let mut ls_data = LS_INSTANTIATION_DATA.load(deps.storage)?;
            ls_data.clock_address = clock_addr;
            // TODO: format autopilot here
            ls_data.lp_address = response.contract_address;

            let ls_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: ls_code,
                msg: to_binary(&ls_data)?,
                funds: vec![],
                label: "covenant-ls".to_string(),
            });

            Ok(Response::default()
                .add_submessage(SubMsg::reply_on_success(ls_instantiate_tx, LS_REPLY_ID)))
        }
        Err(err) => Err(ContractError::Std(StdError::GenericErr {
            msg: err.to_string(),
        })),
    }
}

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            COVENANT_LS_ADDR.save(deps.storage, &response.contract_address)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let lp_addr = COVENANT_LP_ADDR.load(deps.storage)?;
            let depositor_code = DEPOSITOR_CODE.load(deps.storage)?;
            let mut depositor_data = DEPOSITOR_INSTANTIATION_DATA.load(deps.storage)?;
            depositor_data.clock_address = clock_addr;
            depositor_data.atom_receiver.address = lp_addr;
            // st_atom receiver gets queried on demand in depositor

            let depositor_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: depositor_code,
                msg: to_binary(&depositor_data)?,
                funds: vec![],
                label: "covenant-depositor".to_string(),
            });

            Ok(Response::default().add_submessage(SubMsg::reply_on_success(
                depositor_instantiate_tx,
                DEPOSITOR_REPLY_ID,
            )))
        }
        Err(err) => Err(ContractError::Std(StdError::GenericErr {
            msg: err.to_string(),
        })),
    }
}
