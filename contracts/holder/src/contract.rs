#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Coin, BankMsg
};
use cw2::set_contract_version;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{WITHDRAWER};
use crate::error::ContractError;

const CONTRACT_NAME: &str = "crates.io:covenant-holder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // We cannot deserialize the address without first validating it
    let withdrawer = msg
        .withdrawer
        .map(|human| deps.api.addr_validate(&human))
        .transpose()?;
    match &withdrawer {
        // If there is a withdrawer, save it to state
        Some(withdrawer) => WITHDRAWER.save(deps.storage, &withdrawer)?,
        // Error if no withdrawer
        None => return Err(ContractError::NoInitialWithdrawer {}),
    }

    Ok(Response::default().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Withdrawer {} => Ok(
            to_binary(&WITHDRAWER.may_load(deps.storage)?)?
        )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        Withdraw {quantity}=> withdraw(deps, env, info, quantity),
    }
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    quantity: Option<Vec<Coin>>,
    ) -> Result<Response,ContractError> {
    let withdrawer = WITHDRAWER.load(deps.storage)?;

    // Check if the sender is the withdrawer
    if info.sender != withdrawer {
        return Err(ContractError::Unauthorized {});
    }
    // if quantity is specified
    let amount = if let Some(quantity) = quantity {
        quantity
    } else {
        // withdraw everything
        // Querier guarantees to return up-to-date data, including funds sent in this handle message
        // https://github.com/CosmWasm/wasmd/blob/master/x/wasm/internal/keeper/keeper.go#L185-L192
        deps.querier.query_all_balances(&env.contract.address)?
    };
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: withdrawer.to_string(),
            amount,
        })
        .add_attribute("method", "withdraw")
    )
}