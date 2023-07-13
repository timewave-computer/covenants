use astroport::pair::Cw20HookMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg, CosmosMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{WITHDRAWER, LP_ADDRESS};

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
    deps.api.debug("WASMDEBUG: holder instantiate");

    // We cannot deserialize the address without first validating it
    let withdrawer = msg
        .withdrawer
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    match withdrawer {
        // If there is a withdrawer, save it to state
        Some(addr) => WITHDRAWER.save(deps.storage, &addr)?,
        // Error if no withdrawer
        None => return Err(ContractError::NoInitialWithdrawer {}),
    }
    LP_ADDRESS.save(deps.storage, &msg.lp_address)?;

    Ok(Response::default().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Withdrawer {} => Ok(to_binary(&WITHDRAWER.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Withdraw { quantity } => withdraw(deps, env, info, quantity),
    }
}

// /// should be sent to the LP token contract associated with the pool
// /// to withdraw liquidity from
fn try_withdraw_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: withdrawing liquidity");
    // TODO: validate admin    
    let lp_address = LP_ADDRESS.load(deps.storage)?;
    let resp =     deps.querier.query_wasm_raw(lp_address.to_string(), b"pair: {}");
    println!("resp {:?}", resp);

    let pair_info: astroport::asset::PairInfo = deps.querier.query_wasm_smart(
        lp_address.to_string(),
        &astroport::pair::QueryMsg::Pair {},
    )?;

    let liquidity_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        pair_info.liquidity_token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: lp_address,
        amount: liquidity_token_balance.balance,
        msg: to_binary(withdraw_liquidity_hook)?,
    };

    Ok(
        Response::default()
            .add_attribute("method", "try_withdraw")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_info.liquidity_token.to_string(),
                msg: to_binary(withdraw_msg)?,
                funds: vec![],
            })),
    )
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    quantity: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
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
        deps.querier.query_all_balances(env.contract.address)?
    };
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: withdrawer.to_string(),
            amount,
        })
        .add_attribute("method", "withdraw"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateWithdrawer { withdrawer } => {
            let withdrawer_addr = deps.api.addr_validate(&withdrawer)?;
            WITHDRAWER.save(deps.storage, &withdrawer_addr)?;

            Ok(Response::default())
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
        MigrateMsg::WithdrawLiquidity {  } => try_withdraw_liquidity(deps, env),
    }
}
