#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint64, WasmMsg
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{PAUSED, QUEUE, TICK_MAX_GAS, WHITELIST};

const CONTRACT_NAME: &str = "crates.io:covenant-clock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MIN_TICK_MAX_GAS: Uint64 = Uint64::new(200_000);
pub const DEFAULT_TICK_MAX_GAS: Uint64 = Uint64::new(2_900_000);
pub const MAX_TICK_MAX_GAS: Uint64 = Uint64::new(3_000_000);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let tick_max_gas = if let Some(tick_max_gas) = msg.tick_max_gas {
        // at least MIN_MAX_GAS, at most the relayer limit
        tick_max_gas.max(MIN_TICK_MAX_GAS).min(MAX_TICK_MAX_GAS)
    } else {
        // todo: find some reasonable default value
        DEFAULT_TICK_MAX_GAS
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    TICK_MAX_GAS.save(deps.storage, &tick_max_gas)?;
    PAUSED.save(deps.storage, &false)?;

    let whitelist: Vec<Addr> = msg
        .whitelist
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<Vec<Addr>>>()?;

    WHITELIST.save(deps.storage, &whitelist)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("tick_max_gas", tick_max_gas))
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
                                msg: to_json_binary(&ExecuteMsg::Tick {})?,
                                funds: vec![],
                            },
                            0,
                        )
                        .with_gas_limit(TICK_MAX_GAS.load(deps.storage)?.u64()),
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
            // Make sure the caller is whitelisted
            if !WHITELIST
                .load(deps.storage)?
                .iter()
                .any(|a| a == info.sender)
            {
                return Err(ContractError::NotWhitelisted);
            }
            // Make sure the caller is a contract
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
            to_json_binary(&QUEUE.has(deps.storage, Addr::unchecked(address)))
        }
        QueryMsg::Queue { start_after, limit } => to_json_binary(
            &QUEUE.query_queue(
                deps.storage,
                start_after
                    .map(|a| deps.api.addr_validate(&a))
                    .transpose()?,
                limit,
            )?,
        ),
        QueryMsg::TickMaxGas {} => to_json_binary(&TICK_MAX_GAS.load(deps.storage)?),
        QueryMsg::Paused {} => to_json_binary(&PAUSED.load(deps.storage)?),
        QueryMsg::Whitelist {} => to_json_binary(&WHITELIST.load(deps.storage)?),
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

            TICK_MAX_GAS.save(
                deps.storage,
                &new_value.max(MIN_TICK_MAX_GAS).min(MAX_TICK_MAX_GAS),
            )?;
            Ok(Response::default()
                .add_attribute("method", "migrate_update_tick_max_gas")
                .add_attribute("tick_max_gas", new_value))
        }
        MigrateMsg::ManageWhitelist { add, remove } => {
            if add.is_none() && remove.is_none() {
                return Err(ContractError::MustProvideAddOrRemove);
            }

            let mut whitelist = WHITELIST.load(deps.storage)?;

            // Remove addrs from the whitelist if exists, and dequeue them
            if let Some(addrs) = remove {
                for addr in addrs {
                    if let Some(index) = whitelist.iter().position(|x| x == &addr) {
                        QUEUE.remove(deps.storage, whitelist[index].clone())?;
                        whitelist.swap_remove(index);
                    }
                }
            }

            // Add addr if doesn't exist and enqueue them
            if let Some(addrs) = add {
                for addr in addrs {
                    if !whitelist.iter().any(|x| x == &addr) {
                        let addr = deps.api.addr_validate(&addr)?;

                        deps.querier
                            .query_wasm_contract_info(addr.as_str())
                            .map_err(|e| ContractError::NotContract(e.to_string()))?;

                        QUEUE.enqueue(deps.storage, addr.clone())?;
                        whitelist.push(addr);
                    }
                }
            }

            WHITELIST.save(deps.storage, &whitelist)?;

            Ok(Response::default())
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            let version: Version = match CONTRACT_VERSION.parse() {
                Ok(v) => v,
                Err(e) => return Err(ContractError::Std(StdError::generic_err(e.to_string()))),
            };

            let storage_version: Version = match get_contract_version(deps.storage)?.version.parse() {
                Ok(v) => v,
                Err(e) => return Err(ContractError::Std(StdError::generic_err(e.to_string()))),
            };
            if storage_version < version {
                set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            }
            Ok(Response::new())
        }
    }
}
