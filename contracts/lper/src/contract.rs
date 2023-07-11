#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, Uint128, WasmMsg, Decimal, SubMsg, StdError, 
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
    state::{ASSETS, AUTOSTAKE, HOLDER_ADDRESS, LP_POSITION, SLIPPAGE_TOLERANCE, SINGLE_SIDE_LP_LIMIT, PROVIDED_LIQUIDITY_INFO, ProvidedLiquidityInfo},
};

use neutron_sdk::{NeutronResult};

use crate::state::{ContractState, CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-lp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// type QueryDeps<'a> = Deps<'a, NeutronQuery>;
// type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;
const DOUBLE_SIDED_REPLY_ID: u64 = 321u64;
const SINGLE_SIDED_REPLY_ID: u64 = 322u64;


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
    ASSETS.save(deps.storage, &msg.assets)?;
    SINGLE_SIDE_LP_LIMIT.save(deps.storage, &msg.single_side_lp_limit)?;
    PROVIDED_LIQUIDITY_INFO.save(deps.storage, &ProvidedLiquidityInfo {
        provided_amount_ls: Uint128::zero(),
        provided_amount_native: Uint128::zero(),
        leftover_asset: None,
        leftover_asset_counterpart_info: None,
    })?;

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
    println!("\n tick state: {:?}", current_state);
    match current_state {
        ContractState::Instantiated => try_enter_double_side_lp_position(deps, env, info),
        ContractState::DoubleSideLPed => try_enter_single_side_lp_position(deps, env, info),
        ContractState::SingleSideLPed => try_completed(deps),
        ContractState::WithdrawComplete => no_op(),
        _ => no_op(),
    }
}

fn no_op() -> Result<Response, ContractError> {
    Ok(Response::default())
}

fn get_relevant_balances(coins: Vec<Coin>, ls_denom: String, native_denom: String) -> (Coin, Coin) {
    let mut native_bal = Coin::default();
    let mut ls_bal = Coin::default();

    coins.into_iter()
        .for_each(|c| {
            if c.denom == ls_denom {
                // found ls balance
                ls_bal = c;
            } else if c.denom == native_denom {
                if native_denom == c.denom {
                    // found native token balance
                    native_bal = c;
                }
            }
        });

    (native_bal, ls_bal)
}

// here we try to provide double sided liquidity.
fn try_enter_double_side_lp_position(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let asset_data = ASSETS.load(deps.storage)?;
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;

    let bal_coins = deps.querier.query_all_balances(env.contract.address)?;

    // First we filter out non-relevant token balances
    let (mut native_bal, mut ls_bal) = get_relevant_balances(
        bal_coins, 
        asset_data.clone().ls_asset_denom, 
        asset_data.clone().try_get_native_asset_denom().unwrap_or_default()
    );

    // check if we already received the expected amount of native asset.
    // if we have not, we requeue ourselves and wait for funds to arrive.
    if native_bal.amount < asset_data.native_asset_info.amount {
        let enqueue_clock_msg = covenant_clock::helpers::enqueue_msg(clock_addr.as_str())?;
        return Ok(Response::default().add_message(
            CosmosMsg::Wasm(enqueue_clock_msg),
        ))
    }
    
    // we run the simulation and see how much of asset two we need to provide.
    let simulation: SimulationResponse = deps.querier.query_wasm_smart(
        &pool_address.addr,
        &astroport::pair::QueryMsg::Simulation {
            offer_asset: asset_data.native_asset_info.clone(),
            ask_asset_info: None,
        },
    )?;

    // Given a SimulationResponse, we have two possible cases:
    // Case 1: The ask_amount of asset two, returned by simulation is less than the current balance of asset_two
    let mut submsg: SubMsg = if simulation.return_amount < ls_bal.amount {
        // This means that we will have left over asset two, if we are to provide double sided liquidity
        // with the simulation ratio. 

        let ls_asset_double_sided = Asset { 
            info: AssetInfo::NativeToken { denom: asset_data.ls_asset_denom.to_string() },
            // we provide as much as needed to keep in balance with the queried amount
            amount: simulation.return_amount
        };
        let double_sided_liq_msg = ProvideLiquidity {
            assets: vec![
                asset_data.native_asset_info.clone(),
                ls_asset_double_sided.clone(),
            ],
            slippage_tolerance,
            auto_stake,
            receiver: None,
        };

        // convert Asset to Coin types
        let (native_coin, ls_coin) = (asset_data.native_asset_info.to_coin()?, ls_asset_double_sided.clone().to_coin()?);

        // update the provided amounts and leftover assets
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_ls = info.provided_amount_ls.checked_add(ls_coin.clone().amount)?;
            info.provided_amount_native = info.provided_amount_native.checked_add(native_coin.clone().amount)?;
            info.leftover_asset = Some(Asset {
                info: ls_asset_double_sided.info,
                amount: ls_bal.amount.checked_sub(ls_coin.amount)?,
            });
            info.leftover_asset_counterpart_info = Some(asset_data.clone().native_asset_info.info);
            Ok(info)
        })?;

        SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr.to_string(),
                msg: to_binary(&double_sided_liq_msg)?,
                funds: vec![
                    native_coin,
                    ls_coin,
                ],
            }),
            DOUBLE_SIDED_REPLY_ID,
        )
    } else { 
        // Case 2: The ask_amount of asset two, returned by simulation is greater than the current balance of asset_two

        // This means that we will have leftover asset one after providing double sided liquidity with 
        // the total amount of asset two along with the required amount of asset one.
        
        // We first figure out the amount of asset one to be used with proportions:
        // native asset amount / ls asset simulation return = x / available ls amount
        // x = available ls amount * native asset amount / ls asset simulation return
        let native_asset_amt = ls_bal.amount * native_bal.amount / simulation.return_amount;

        let mut double_sided_native_asset = asset_data.native_asset_info.clone();        
        double_sided_native_asset.amount = native_asset_amt;
        let mut double_sided_ls_asset = Asset {
            info: AssetInfo::NativeToken { denom: asset_data.ls_asset_denom, },
            amount: ls_bal.amount,
        };

        // We should provide double sided liquidity regardless of left over.
        let double_sided_liq_msg = ProvideLiquidity {
            assets: vec![
                double_sided_ls_asset.clone(),
                double_sided_native_asset.clone(),
            ],
            slippage_tolerance,
            auto_stake,
            receiver: None,
        };

        // convert Asset to Coin types
        let (native_coin, ls_coin) = (double_sided_native_asset.to_coin()?, double_sided_ls_asset.to_coin()?);

        // update the provided amounts and leftover assets
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_ls = info.provided_amount_ls.checked_add(ls_coin.clone().amount)?;
            info.provided_amount_native = info.provided_amount_native.checked_add(native_coin.clone().amount)?;
            info.leftover_asset = Some(Asset {
                info: double_sided_native_asset.info,
                amount: native_bal.amount.checked_sub(native_coin.amount)?,
            });
            info.leftover_asset_counterpart_info = Some(double_sided_ls_asset.info);
            Ok(info)
        })?;

        SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr.to_string(),
                msg: to_binary(&double_sided_liq_msg)?,
                funds: vec![
                    native_coin,
                    ls_coin,
                ],
            }),
            DOUBLE_SIDED_REPLY_ID,
        )
    };

    Ok(Response::default().add_submessage(submsg))
}

fn try_enter_single_side_lp_position(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let asset_data = ASSETS.load(deps.storage)?;
    let max_single_side_ratio: Decimal = SINGLE_SIDE_LP_LIMIT.load(deps.storage)?;
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
    let provided_liquidity_info = PROVIDED_LIQUIDITY_INFO.load(deps.storage)?;

    let bal_coins = deps.querier.query_all_balances(env.contract.address)?;

    // First we filter out non-relevant token balances
    let (mut native_bal, mut ls_bal) = get_relevant_balances(
        bal_coins, 
        asset_data.clone().ls_asset_denom, 
        asset_data.clone().try_get_native_asset_denom().unwrap_or_default()
    );

    // assume leftover is native asset
    let mut leftover_asset = asset_data.clone().native_asset_info;

    // if there is some leftover asset stored..
    let leftover_ratio = if let Some(asset) = provided_liquidity_info.leftover_asset {
        leftover_asset = asset.clone();
        // if there is leftover native tokens...
        if asset.info == asset_data.native_asset_info.info {
            Decimal::from_ratio(
                asset.amount,
                provided_liquidity_info.provided_amount_native,
            )
        } else {
            // otherwise there is leftover ls tokens
            Decimal::from_ratio(
                asset.amount,
                provided_liquidity_info.provided_amount_ls,
            )
        }
    } else {
        Decimal::one()
    };

    println!("asset asset: {:?}", leftover_asset);

    // if ratio is within limits, we are ready to provide single-sided liquidity
    if leftover_ratio <= max_single_side_ratio {
        // we see if some counterpart info is stored
        if let Some(counterparty_info) = provided_liquidity_info.leftover_asset_counterpart_info {
            let single_sided_liq_msg = ProvideLiquidity { 
                assets: vec![
                    leftover_asset.clone(),
                    Asset {
                        info: counterparty_info,
                        amount: Uint128::zero(),
                    },
                ],
                slippage_tolerance,
                auto_stake,
                receiver: None,
            };
            let coin = leftover_asset.to_coin()?;
            // update provided amount
            PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
                if coin.denom == asset_data.ls_asset_denom {
                    info.provided_amount_ls = info.provided_amount_ls.checked_add(coin.clone().amount)?;
                } else {
                    info.provided_amount_native = info.provided_amount_native.checked_add(coin.clone().amount)?;
                }
            
                info.leftover_asset = None;
                info.leftover_asset_counterpart_info = None;
                Ok(info)
            })?;
        
            return Ok(Response::default().add_submessage(
                SubMsg::reply_on_success(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pool_address.addr.to_string(),
                    msg: to_binary(&single_sided_liq_msg)?,
                    funds: vec![
                        coin,
                    ],
                }), SINGLE_SIDED_REPLY_ID)
            ))
        }        
    } else {
        // if ratio is exceeded, something went wrong:
        // - we go back to instantiated state and expect to be funded with 
        //   some of the counterparty asset in order to provide all liquidity
        CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    }

    Ok(Response::default())
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
fn try_completed(deps: DepsMut) -> Result<Response, ContractError> {
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
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    println!("{:?}", msg.clone().result.unwrap());
    match msg.id {
        DOUBLE_SIDED_REPLY_ID => handle_double_sided_reply_id(deps, _env, msg),
        SINGLE_SIDED_REPLY_ID => handle_single_sided_reply_id(deps, _env, msg),
        _ => Err(ContractError::from(StdError::GenericErr { msg: "err".to_string() }))
    }
}

fn handle_double_sided_reply_id(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    CONTRACT_STATE.save(deps.storage, &ContractState::DoubleSideLPed)?;

    Ok(Response::default()
        .add_attribute("method", "handle_double_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string())   
    )
}

fn handle_single_sided_reply_id(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    CONTRACT_STATE.save(deps.storage, &ContractState::SingleSideLPed)?;

    Ok(Response::default()
        .add_attribute("method", "handle_single_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string())   
    )}