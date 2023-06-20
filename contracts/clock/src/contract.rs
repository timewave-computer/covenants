#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint64, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{PAUSED, QUEUE, TICK_MAX_GAS};

const CONTRACT_NAME: &str = "crates.io:covenant-clock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.tick_max_gas.is_zero() {
        return Err(ContractError::ZeroTickMaxGas {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    TICK_MAX_GAS.save(deps.storage, &msg.tick_max_gas.u64())?;
    PAUSED.save(deps.storage, &false)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("tick_max_gas", msg.tick_max_gas))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let is_paused = PAUSED.load(deps.storage)?;
    if is_paused {
        return Err(ContractError::Paused {});
    }

    match msg {
        ExecuteMsg::Tick {} => {
            if let Some(receiver) = QUEUE.dequeue(deps.storage)? {
                QUEUE.enqueue(deps.storage, receiver.clone())?;
                Ok(Response::default()
                    .add_attribute("method", "execute_tick")
                    .add_attribute("dequeued", receiver.as_str())
                    .add_submessage(
                        SubMsg::reply_on_error(
                            WasmMsg::Execute {
                                contract_addr: receiver.to_string(),
                                msg: to_binary(&ExecuteMsg::Tick {})?,
                                funds: vec![],
                            },
                            0,
                        )
                        .with_gas_limit(TICK_MAX_GAS.load(deps.storage)?),
                    ))
            } else {
                Ok(Response::default()
                    .add_attribute("method", "execute_tick")
                    .add_attribute("dequeued", "none"))
            }
        }
        ExecuteMsg::Enqueue {} => {
            if QUEUE.has(deps.storage, info.sender.clone()) {
                return Err(ContractError::AlreadyEnqueued);
            }
            deps.querier
                .query_wasm_contract_info(info.sender.as_str())
                .map_err(|e| ContractError::NotContract(e.to_string()))?;

            QUEUE.enqueue(deps.storage, info.sender.clone())?;
            Ok(Response::default()
                .add_attribute("method", "execute_enqueue")
                .add_attribute("sender", info.sender))
        }
        ExecuteMsg::Dequeue {} => {
            QUEUE.remove(deps.storage, info.sender.clone())?;
            Ok(Response::default()
                .add_attribute("method", "execute_dequeue")
                .add_attribute("sender", info.sender))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsQueued { address } => {
            to_binary(&QUEUE.has(deps.storage, Addr::unchecked(address)))
        }
        QueryMsg::Queue { start_after, limit } => to_binary(
            &QUEUE.query_queue(
                deps.storage,
                start_after
                    .map(|a| deps.api.addr_validate(&a))
                    .transpose()?,
                limit,
            )?,
        ),
        QueryMsg::TickMaxGas {} => to_binary(&Uint64::new(TICK_MAX_GAS.load(deps.storage)?)),
        QueryMsg::Paused {} => to_binary(&PAUSED.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // Tick messages are dispatched with reply ID 0 and reply on
    // error. If an error occurs, we ignore it but stop the parent
    // message from failing, so the state change which moved the tick
    // receiver to the end of the message queue gets committed. This
    // prevents an erroring tick receiver from locking the clock.
    if msg.id != 0 {
        Err(ContractError::UnexpectedReplyId(msg.id))
    } else {
        Ok(Response::default()
            .add_attribute("method", "reply_on_error")
            .add_attribute("error", msg.result.unwrap_err()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::Pause {} => {
            let is_paused = PAUSED.load(deps.storage)?;
            if is_paused {
                return Err(ContractError::Paused {});
            }
            PAUSED.save(deps.storage, &true)?;
            Ok(Response::default().add_attribute("method", "migrate_pause"))
        }
        MigrateMsg::Unpause {} => {
            let is_paused = PAUSED.load(deps.storage)?;
            if !is_paused {
                return Err(ContractError::NotPaused {});
            }
            PAUSED.save(deps.storage, &false)?;
            Ok(Response::default().add_attribute("method", "migrate_unpause"))
        }
        MigrateMsg::UpdateTickMaxGas { new_value } => {
            if new_value.is_zero() {
                return Err(ContractError::ZeroTickMaxGas {});
            }
            TICK_MAX_GAS.save(deps.storage, &new_value.u64())?;
            Ok(Response::default()
                .add_attribute("method", "migrate_update_tick_max_gas")
                .add_attribute("tick_max_gas", new_value))
        }
    }
}
