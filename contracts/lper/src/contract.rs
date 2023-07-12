#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, Uint128, WasmMsg,
};
use covenant_clock::helpers::verify_clock;
use cw2::set_contract_version;

use astroport::{
    asset::{Asset, AssetInfo},
    pair::{Cw20HookMsg, ExecuteMsg::ProvideLiquidity, SimulationResponse},
};
use cw20::{BalanceResponse, Cw20ExecuteMsg};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{ASSETS, AUTOSTAKE, HOLDER_ADDRESS, LP_POSITION, SLIPPAGE_TOLERANCE},
};

use neutron_sdk::{bindings::msg::NeutronMsg, NeutronResult};

use crate::state::{ContractState, CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-lp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// type QueryDeps<'a> = Deps<'a, NeutronQuery>;
// type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: lp instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    //enqueue clock
    CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&msg.clock_address)?)?;
    let clock_enqueue_msg = covenant_clock::helpers::enqueue_msg(&msg.clock_address)?;

    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    LP_POSITION.save(deps.storage, &msg.lp_position)?;
    HOLDER_ADDRESS.save(deps.storage, &msg.holder_address)?;
    let assets: Vec<Asset> = msg.assets.into_iter().collect();

    ASSETS.save(deps.storage, &assets)?;

    Ok(Response::default().add_message(clock_enqueue_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // validate clock
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::WithdrawLiquidity {} => try_withdraw(deps, env, info),
    }
}

fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    let is_clock = verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?);
    match is_clock {
        Ok(_) => (),
        Err(_err) => return Err(ContractError::ClockVerificationError {}),
    }

    let current_state = CONTRACT_STATE.load(deps.storage)?;

    match current_state {
        ContractState::Instantiated => try_enter_lp_position(deps, env, info),
        ContractState::LpPositionEntered => no_op(),
        ContractState::LpPositionExited => no_op(),
        ContractState::WithdrawComplete => no_op(),
    }
}

fn no_op() -> Result<Response, ContractError> {
    Ok(Response::default())
}

fn try_enter_lp_position(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let assets = ASSETS.load(deps.storage)?;
    let first_asset = &assets[0];

    // figure out how much of second asset can we get with one of first asset
    let simulation: SimulationResponse = deps.querier.query_wasm_smart(
        &pool_address.addr,
        &astroport::pair::QueryMsg::Simulation {
            offer_asset: first_asset.to_owned(),
            ask_asset_info: Some(assets[1].info.to_owned()),
        },
    )?;

    let (leftover_asset, leftover_asset_counterpart) =
        if first_asset.amount > simulation.return_amount {
            (
                Asset {
                    info: assets[1].clone().info,
                    amount: first_asset.amount - simulation.return_amount,
                },
                Asset {
                    info: assets[0].clone().info,
                    amount: Uint128::zero(),
                },
            )
        } else {
            (
                Asset {
                    info: assets[0].clone().info,
                    amount: simulation.return_amount - first_asset.amount,
                },
                Asset {
                    info: assets[1].clone().info,
                    amount: Uint128::zero(),
                },
            )
        };
    let (leftover_bal, _leftover_bal_counterpart) = (
        leftover_asset.to_coin()?,
        leftover_asset_counterpart.to_coin()?,
    );

    deps.api.debug(&format!(
        "\nWASMDEBUG: {:?}{:?} = {:?}{:?}\n",
        first_asset.amount,
        first_asset.to_coin()?.denom,
        simulation.return_amount,
        assets[1].to_coin()?.denom
    ));
    deps.api
        .debug(&format!("WASMDEBUG: leftover asset: {:?}\n", leftover_bal));

    println!(
        "\n coin balances: {:?}",
        deps.querier
            .query_all_balances(env.contract.clone().address)?
    );

    let balances: Vec<Coin> = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|coin| {
            let mut valid_balance = false;
            for asset in assets.clone() {
                match asset.info {
                    AssetInfo::Token { contract_addr } => {
                        if coin.denom == contract_addr {
                            valid_balance = false
                        }
                    }
                    AssetInfo::NativeToken { denom } => {
                        if coin.denom == denom {
                            valid_balance = true
                        }
                    }
                }
            }
            valid_balance
        })
        .map(|mut c| {
            // convert balances according to simulation
            if c.denom == leftover_bal.denom {
                // if its the coin with leftovers, subtract
                c.amount -= leftover_bal.amount;
            }
            c
        })
        .collect();

    println!("\n after sim coin balances: {:?}", balances);

    // generate astroport Assets from balances
    let assets: Vec<Asset> = balances
        .clone()
        .into_iter()
        .map(|bal| Asset {
            info: AssetInfo::NativeToken { denom: bal.denom },
            amount: bal.amount,
        })
        .collect();

    let provide_liquidity_msg = ProvideLiquidity {
        assets,
        slippage_tolerance,
        auto_stake,
        receiver: None,
    };

    // We can safely dequeue the clock here
    // if PL fails, dequeue wont happen and we will just try again.
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
    let dequeue_clock_msg = covenant_clock::helpers::dequeue_msg(clock_addr.as_str())?;

    let single_sided_liq_msg = ProvideLiquidity {
        assets: vec![leftover_asset, leftover_asset_counterpart],
        slippage_tolerance,
        auto_stake,
        receiver: None,
    };

    Ok(Response::default().add_messages(vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_address.addr.to_string(),
            msg: to_binary(&provide_liquidity_msg)?,
            funds: balances,
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_address.addr,
            msg: to_binary(&single_sided_liq_msg)?,
            funds: vec![leftover_bal],
        }),
        CosmosMsg::Wasm(dequeue_clock_msg),
    ]))
}

/// should be sent to the LP token contract associated with the pool
/// to withdraw liquidity from
fn try_withdraw(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let assets = ASSETS.load(deps.storage)?;
    deps.api.debug(&format!("withdraw assets: {:?}", assets));

    let pair_info: astroport::asset::PairInfo = deps.querier.query_wasm_smart(
        pool_address.addr.to_string(),
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
        contract: pool_address.addr,
        amount: liquidity_token_balance.balance,
        msg: to_binary(withdraw_liquidity_hook)?,
    };

    Ok(
        Response::default().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: to_binary(withdraw_msg)?,
            funds: vec![],
        })),
    )
}

#[allow(unused)]
fn try_completed(deps: DepsMut) -> NeutronResult<Response<NeutronMsg>> {
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
    let msg = covenant_clock::helpers::dequeue_msg(clock_addr.as_str())?;

    Ok(Response::default().add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::LpPosition {} => Ok(to_binary(&LP_POSITION.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::Assets {} => Ok(to_binary(&ASSETS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> NeutronResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            lp_position,
            holder_address,
        } => {
            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
            }

            if let Some(lp_position) = lp_position {
                LP_POSITION.save(deps.storage, &lp_position)?;
            }

            if let Some(holder_address) = holder_address {
                HOLDER_ADDRESS.save(deps.storage, &holder_address)?;
            }

            Ok(Response::default())
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    Ok(Response::new())
}
