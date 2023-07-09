#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, Uint128, WasmMsg, Decimal, SubMsg,
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
    state::{ASSETS, AUTOSTAKE, HOLDER_ADDRESS, LP_POSITION, SLIPPAGE_TOLERANCE, SINGLE_SIDE_LP_LIMIT},
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
    ASSETS.save(deps.storage, &msg.assets)?;
    SINGLE_SIDE_LP_LIMIT.save(deps.storage, &msg.single_side_lp_limit)?;

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
    let asset_data = ASSETS.load(deps.storage)?;
    let max_single_side_ratio: Decimal = SINGLE_SIDE_LP_LIMIT.load(deps.storage)?;
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;

    // First we filter out non-relevant token balances
    let mut native_bal = Coin::default();
    let mut ls_bal = Coin::default();
    deps.querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .for_each(|c| {
            if c.denom == asset_data.ls_asset_denom {
                // found ls balance
                ls_bal = c;
            } else if let Some(native_denom) = asset_data.clone().try_get_native_asset_denom() {
                if native_denom == c.denom {
                    // found native token balance
                    native_bal = c;
                }
            }
        });
    
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

    let mut submessages: Vec<CosmosMsg> = Vec::new();
    // Given a SimulationResponse, we have two possible cases:

    // Case 1: The ask_amount of asset two, returned by simulation is less than the current balance of asset_two
    if simulation.return_amount < ls_bal.amount {
        // This means that we will have left over asset two, if we are to provide double sided liquidity
        // with the simulation ratio. 

        // We should provide double sided liquidity regardless of left over.
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
        submessages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_address.addr.to_string(),
            msg: to_binary(&double_sided_liq_msg)?,
            funds: vec![
                asset_data.native_asset_info.to_coin()?,
                ls_asset_double_sided.clone().to_coin()?,
            ],
        }));
        
        // If there is no left over, we can go to completion.
        // We can get the left_over_asset_two: balance of asset two - ask amount
        let left_over_ls_amount = ls_bal.amount - simulation.return_amount;

        // If the left_over_asset_two / current balance of asset two <= max single sided liquidity ratio
        // We should provide single sided liquidity and we can go to completion state.
        let left_over_to_available_bal_ratio = Decimal::from_ratio(
            left_over_ls_amount, 
            ls_bal.amount
        );       
        if left_over_to_available_bal_ratio <= max_single_side_ratio {
            let mut native_asset = asset_data.native_asset_info.clone();
            native_asset.amount = Uint128::zero();
            let ls_single_sided_asset = Asset { 
                info: AssetInfo::NativeToken { denom: asset_data.ls_asset_denom.to_string() },
                amount: left_over_ls_amount,
            };
            let single_sided_liq_msg = ProvideLiquidity { 
                assets: vec![
                    ls_single_sided_asset.clone(),
                    native_asset,
                ],
                slippage_tolerance,
                auto_stake,
                receiver: None,
            };
            submessages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr.to_string(),
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![
                    ls_single_sided_asset.to_coin()?,
                ],
            }));
        }
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
        submessages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_address.addr.to_string(),
            msg: to_binary(&double_sided_liq_msg)?,
            funds: vec![
                double_sided_native_asset.to_coin()?,
                double_sided_ls_asset.to_coin()?,
            ],
        }));

        // If there is no left over, we can go to completion.
        // We can get the left_over_asset_two: balance of asset two - ask amount
        let native_coin_leftover = native_bal.amount - double_sided_native_asset.amount;

        // If the left_over_asset_two / current balance of asset two <= max single sided liquidity ratio
        // We should provide single sided liquidity and we can go to completion state.
        let left_over_to_available_bal_ratio = Decimal::from_ratio(
            native_coin_leftover, 
            native_bal.amount
        );       
        if left_over_to_available_bal_ratio <= max_single_side_ratio {
            let mut ls_asset = double_sided_ls_asset.clone();
            ls_asset.amount = Uint128::zero();

            let native_single_sided_asset = Asset { 
                info: asset_data.native_asset_info.clone().info,
                amount: native_coin_leftover,
            };
            let single_sided_liq_msg = ProvideLiquidity { 
                assets: vec![
                    ls_asset.clone(),
                    native_single_sided_asset.clone(),
                ],
                slippage_tolerance,
                auto_stake,
                receiver: None,
            };
            submessages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr.to_string(),
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![
                    native_single_sided_asset.to_coin()?,
                ],
            }));
        }
    }


    // TODO: enque/deque
    // TODO: return message responses?


    Ok(Response::default())
}

fn validate_single_sided_liquidity_amount(
    single_side_amount: Uint128,
    normal_amount: Uint128,
    max_single_side_ratio: Decimal
) -> Result<(), ContractError> {
    let ratio = Decimal::from_ratio(single_side_amount, normal_amount);
    println!("ratio {:?}", ratio);
    if ratio > max_single_side_ratio {
        return Err(ContractError::SingleSideLpLimitError {})
    } else {
        return Ok(())
    }
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
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    Ok(Response::new())
}
