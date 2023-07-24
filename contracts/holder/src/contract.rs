use astroport::pair::Cw20HookMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{WITHDRAWER, POOL_ADDRESS};

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
    let mut resp = Response::default().add_attribute("method", "instantiate");

    // withdrawer is optional on instantiation; can be set later
    if let Some(addr) = msg.withdrawer {
        WITHDRAWER.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
        resp = resp.add_attribute("withdrawer", addr);
    };

    let pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    POOL_ADDRESS.save(deps.storage, &pool_addr)?;

    Ok(resp.add_attribute("pool_address", pool_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Withdrawer {} => Ok(to_binary(&WITHDRAWER.may_load(deps.storage)?)?),
        QueryMsg::PoolAddress {} => Ok(to_binary(&POOL_ADDRESS.may_load(deps.storage)?)?),
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
        ExecuteMsg::WithdrawLiquidity {} => try_withdraw_liquidity(deps, env, info),
        ExecuteMsg::Withdraw { quantity } => try_withdraw_balances(deps, env, info, quantity),
    }
}

/// tries to remove liquidity from the pool. withdrawer has to be set for this to work.
/// withdrawer is also the only permitted caller of this method.
/// works by querying the pool for the amount of LP tokens held by this contract.
/// then it submits a `WithdrawLiquidity` hook to the pool which in turn
/// burns the LP tokens and credits this contract with the underlying assets.
fn try_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: withdrawing liquidity");

    // withdrawer has to be set for initiating liquidity withdrawal
    let withdrawer = if let Some(addr) = WITHDRAWER.may_load(deps.storage)? {
        addr
    } else {
        return Err(ContractError::NoWithdrawerError {});
    };

    // we validate who is initiating the liquidity removal
    if withdrawer != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let pool_address = POOL_ADDRESS.load(deps.storage)?;

    // We query the pool to get the contract for the pool info
    // The pool info is required to fetch the address of the
    // liquidity token contract. The liquidity tokens are CW20 tokens
    let pair_info: astroport::asset::PairInfo = deps
        .querier
        .query_wasm_smart(pool_address.to_string(), &astroport::pair::QueryMsg::Pair {})?;

    // We query our own liquidity token balance
    let liquidity_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        pair_info.clone().liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    // We withdraw our liquidity constructing a CW20 send message
    // The message contains our liquidity token balance
    // The pool address and a message to call the withdraw liquidity hook of the pool contract
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool_address.to_string(),
        amount: liquidity_token_balance.balance,
        msg: to_binary(withdraw_liquidity_hook)?,
    };
    // We execute the message on the liquidity token contract
    // This will burn the LP tokens and withdraw liquidity into the holder
    Ok(Response::default()
        .add_attribute("method", "try_withdraw")
        .add_attribute("lp_token_amount", liquidity_token_balance.balance)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: to_binary(withdraw_msg)?,
            funds: vec![],
        })))
}

/// tries to withdraw assets from this contract to the withdrawer address.
/// withdrawer has to be set for this to work; it is also the only permitted caller.
/// accepts an optional quantity in form of `Vec<Coin>`. if no quantity is
/// provided, all assets are withdrawn. otherwise only the specified ones.
pub fn try_withdraw_balances(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    quantity: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
    // withdrawer has to be set for initiating balance withdrawal
    let withdrawer = if let Some(addr) = WITHDRAWER.may_load(deps.storage)? {
        addr
    } else {
        return Err(ContractError::NoWithdrawerError {});
    };

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
        .add_attribute("method", "try_withdraw")
        .add_message(BankMsg::Send {
            to_address: withdrawer.to_string(),
            amount,
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            withdrawer,
            pool_address,
        } => {
            let mut response = Response::default().add_attribute("method", "update_withdrawer");

            if let Some(addr) = withdrawer {
                WITHDRAWER.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("withdrawer", addr);
            }

            if let Some(addr) = pool_address {
                POOL_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("pool_address", addr);
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default().add_attribute("method", "update_withdrawer"))
        }
    }
}
