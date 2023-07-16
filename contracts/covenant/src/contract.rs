#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, WasmMsg,
};

use cw2::set_contract_version;

use crate::{
    error::ContractError,
    instantiate2::get_instantiate_messages,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CLOCK_CODE, COVENANT_CLOCK_ADDR, COVENANT_DEPOSITOR_ADDR, COVENANT_HOLDER_ADDR,
        COVENANT_LP_ADDR, COVENANT_LS_ADDR, DEPOSITOR_CODE, HOLDER_CODE, LP_CODE, LS_CODE,
        POOL_ADDRESS, IBC_TIMEOUT,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 60 * 24 * 7 * 2;

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

    let ibc_timeout = if let Some(timeout) = msg.ibc_msg_transfer_timeout_timestamp {
        timeout
    } else {
        DEFAULT_TIMEOUT_SECONDS
    };
    IBC_TIMEOUT.save(deps.storage, &ibc_timeout)?;

    let instantiate2_msgs = get_instantiate_messages(deps, env, msg)?;

    Ok(Response::default()
        .add_messages(instantiate2_msgs)
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    Ok(Response::default())
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
