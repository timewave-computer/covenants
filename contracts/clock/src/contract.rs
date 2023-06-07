#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::QUEUE;

const CONTRACT_NAME: &str = "crates.io:covenant-clock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => {
            if let Some(receiver) = QUEUE.dequeue(deps.storage)? {
                QUEUE.enqueue(deps.storage, &env.block, receiver.clone())?;
                Ok(Response::default()
                    .add_attribute("method", "execute_tick")
                    .add_attribute("dequeued", receiver.as_str())
                    .add_submessage(SubMsg::reply_on_error(
                        WasmMsg::Execute {
                            contract_addr: receiver.to_string(),
                            msg: to_binary(&ExecuteMsg::Tick {})?,
                            funds: vec![],
                        },
                        0,
                    )))
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
            QUEUE.enqueue(deps.storage, &env.block, info.sender.clone())?;
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
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != 0 {
        Err(ContractError::UnexpectedReplyId(msg.id))
    } else {
        Ok(Response::default()
            .add_attribute("method", "reply_on_error")
            .add_attribute("error", msg.result.unwrap_err()))
    }
}
