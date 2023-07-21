#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg, Decimal,
};
use covenant_clock::helpers::verify_clock;
use cw2::set_contract_version;

use astroport::{
    asset::{Asset, AssetInfo},
    pair::{ExecuteMsg::ProvideLiquidity, SimulationResponse},
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        ProvidedLiquidityInfo, ASSETS, AUTOSTAKE, HOLDER_ADDRESS, LP_POSITION,
        PROVIDED_LIQUIDITY_INFO, SINGLE_SIDED_LP_LIMITS, SLIPPAGE_TOLERANCE, ALLOWED_RETURN_DELTA, EXPECTED_RETURN_AMOUNT, EXPECTED_NATIVE_TOKEN_AMOUNT,
    },
};

use neutron_sdk::NeutronResult;

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

    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    LP_POSITION.save(deps.storage, &msg.lp_position)?;
    HOLDER_ADDRESS.save(deps.storage, &msg.holder_address)?;
    ASSETS.save(deps.storage, &msg.assets)?;
    SINGLE_SIDED_LP_LIMITS.save(deps.storage, &msg.single_side_lp_limits)?;
    PROVIDED_LIQUIDITY_INFO.save(
        deps.storage,
        &ProvidedLiquidityInfo {
            provided_amount_ls: Uint128::zero(),
            provided_amount_native: Uint128::zero(),
        },
    )?;
    ALLOWED_RETURN_DELTA.save(deps.storage, &msg.allowed_return_delta)?;
    EXPECTED_RETURN_AMOUNT.save(deps.storage, &msg.expected_return_amount)?;
    EXPECTED_NATIVE_TOKEN_AMOUNT.save(deps.storage, &msg.expected_native_token_amount)?;

    Ok(Response::default().add_attribute("method", "instantiate"))
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
    }
}

fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    println!("\n tick state: {:?}", current_state);
    match current_state {
        ContractState::Instantiated => try_lp(deps, env, info),
        ContractState::WithdrawComplete => try_completed(deps),
    }
}

fn try_lp(mut deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let contract_addr = env.contract.address;
    let pool_address = LP_POSITION.load(deps.storage)?;

    // we try to submit a double-sided liquidity message first
    let double_sided_submsg =
        try_get_double_side_lp_submsg(deps.branch(), contract_addr.to_string())?;
    deps.api.debug("Trying to double-side lp...");
    if let Some(msg) = double_sided_submsg {
        return Ok(Response::default()
            .add_submessage(msg)
            .add_attribute("method", "double_side_lp"));
    }
    deps.api.debug("Trying to single-side lp...");
    // if ds msg fails, try to submit a single-sided liquidity message
    let single_sided_submsg =
        try_get_single_side_lp_submsg(deps.branch(), contract_addr.to_string())?;
    if let Some(msg) = single_sided_submsg {
        return Ok(Response::default()
            .add_submessage(msg)
            .add_attribute("method", "single_side_lp"));
    }

    deps.api
        .debug("Neither single nor double-sided liquidity can be provided");
    // if neither worked, we do not advance the state machine and
    // keep waiting for more funds to arrive
    Ok(Response::default()
        .add_attribute("method", "try_lp")
        .add_attribute("status", "not enough funds"))
}

fn try_completed(deps: DepsMut) -> Result<Response, ContractError> {
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
    let msg = covenant_clock::helpers::dequeue_msg(clock_addr.as_str())?;

    Ok(Response::default().add_message(msg))
}

fn get_relevant_balances(coins: Vec<Coin>, ls_denom: String, native_denom: String) -> (Coin, Coin) {
    let mut native_bal = Coin::default();
    let mut ls_bal = Coin::default();

    for c in coins {
        if c.denom == ls_denom {
            // found ls balance
            ls_bal = c;
        } else if c.denom == native_denom {
            // found native token balance
            native_bal = c;
        }
    }

    (native_bal, ls_bal)
}

fn validate_price_range(
    offer_asset_amount: Uint128,
    return_asset_amount: Uint128,
    expected_native_token_amount: Uint128,
    expected_return_amount: Uint128,
    allowed_return_delta: Uint128
) -> Result<(), ContractError> {
    // find the min and max return amounts allowed by deviating away from expected return amount
    // by allowed delta
    println!("exp re amt :{:?}", expected_return_amount);
    let min_return_amount = expected_return_amount - allowed_return_delta;
    let max_return_amount = expected_return_amount + allowed_return_delta;

    // derive allowed proportions
    let min_accepted_ratio = Decimal::from_ratio(min_return_amount, expected_native_token_amount);
    let max_accepted_ratio = Decimal::from_ratio(max_return_amount, expected_native_token_amount);

    // we find the proportion of the price range being validated
    let validation_ratio = Decimal::from_ratio(return_asset_amount, offer_asset_amount);

    // if current return to offer amount ratio falls out of [min_accepted_ratio, max_return_amount],
    // return price range error
    if validation_ratio < min_accepted_ratio || validation_ratio > max_accepted_ratio {
        return Err(ContractError::PriceRangeError {})
    }

    Ok(())
}

// here we try to provide double sided liquidity.
// we don't care about the amounts; just try to provide as much as possible
fn try_get_double_side_lp_submsg(
    deps: DepsMut,
    lp_contract: String,
) -> Result<Option<SubMsg>, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let asset_data = ASSETS.load(deps.storage)?;

    let bal_coins = deps.querier.query_all_balances(lp_contract)?;

    // First we filter out non-relevant token balances
    let (native_bal, ls_bal) = get_relevant_balances(
        bal_coins,
        asset_data.clone().ls_asset_denom,
        asset_data.clone().native_asset_denom,
    );

    // if either of the balances are zero we should provide single sided liquidity; exit
    if native_bal.amount.is_zero() || ls_bal.amount.is_zero() {
        deps.api.debug("Either native or ls balance is zero");
        return Ok(None);
    }
    // we run the simulation and see how much of asset two we need to provide.
    let mut native_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: native_bal.denom,
        },
        amount: native_bal.amount,
    };

    let simulation: SimulationResponse = deps.querier.query_wasm_smart(
        &pool_address.addr,
        &astroport::pair::QueryMsg::Simulation {
            offer_asset: native_asset.clone(),
            ask_asset_info: None,
        },
    )?;

    let expected_return_amount = EXPECTED_RETURN_AMOUNT.load(deps.storage)?;
    let expected_native_token_amount = EXPECTED_NATIVE_TOKEN_AMOUNT.load(deps.storage)?;

    let allowed_return_delta = ALLOWED_RETURN_DELTA.load(deps.storage)?;
    validate_price_range(
        native_asset.clone().amount, 
        simulation.return_amount,
        expected_native_token_amount,
        expected_return_amount,
        allowed_return_delta,
    )?;

    let holder_address = HOLDER_ADDRESS.load(deps.storage)?;

    println!("double side lp sim resp: {:?}", simulation);
    // Given a SimulationResponse, we have two possible cases:
    // Case 1: The ask_amount of asset two, returned by simulation is less than the current balance of asset_two
    if simulation.return_amount <= ls_bal.amount {
        // This means that we will have left over LS tokens, if we are to provide double sided liquidity
        // with the simulation ratio.
        let ls_asset_double_sided = Asset {
            info: asset_data.get_ls_asset_info(),
            // we provide as much as needed to keep in balance with the queried amount
            amount: simulation.return_amount,
        };
        let double_sided_liq_msg = ProvideLiquidity {
            assets: vec![native_asset.clone(), ls_asset_double_sided.clone()],
            slippage_tolerance,
            auto_stake,
            receiver: Some(holder_address),
        };
        println!("double sided liq msg: {:?}", double_sided_liq_msg);

        // convert Asset to Coin types
        let (native_coin, ls_coin) = (native_asset.to_coin()?, ls_asset_double_sided.to_coin()?);

        // update the provided amounts and leftover assets
        PROVIDED_LIQUIDITY_INFO.update(
            deps.storage,
            |mut info: ProvidedLiquidityInfo| -> StdResult<_> {
                info.provided_amount_ls = info
                    .provided_amount_ls
                    .checked_add(ls_coin.clone().amount)?;
                info.provided_amount_native = info
                    .provided_amount_native
                    .checked_add(native_coin.clone().amount)?;
                Ok(info)
            },
        )?;

        Ok(Some(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr,
                msg: to_binary(&double_sided_liq_msg)?,
                funds: vec![native_coin, ls_coin],
            }),
            DOUBLE_SIDED_REPLY_ID,
        )))
    } else {
        // Case 2: The ask_amount of asset two, returned by simulation is greater than the current balance of asset_two

        // This means that we will have leftover asset one after providing double sided liquidity with
        // the total amount of asset two along with the required amount of asset one.

        // We first figure out the amount of asset one to be used with proportions:
        // native asset amount / ls asset simulation return = x / available ls amount
        // x = available ls amount * native asset amount / ls asset simulation return
        let native_asset_amt = ls_bal.amount * native_bal.amount / simulation.return_amount;
        native_asset.amount = native_asset_amt;

        let double_sided_ls_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: asset_data.ls_asset_denom,
            },
            amount: ls_bal.amount,
        };

        // We should provide double sided liquidity regardless of left over.
        let double_sided_liq_msg = ProvideLiquidity {
            assets: vec![double_sided_ls_asset.clone(), native_asset.clone()],
            slippage_tolerance,
            auto_stake,
            receiver: Some(holder_address),
        };
        println!("double sided liq msg: {:?}", double_sided_liq_msg);
        // convert Asset to Coin types
        let (native_coin, ls_coin) = (native_asset.to_coin()?, double_sided_ls_asset.to_coin()?);

        // update the provided amounts and leftover assets
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_ls = info
                .provided_amount_ls
                .checked_add(ls_coin.clone().amount)?;
            info.provided_amount_native = info
                .provided_amount_native
                .checked_add(native_coin.clone().amount)?;
            Ok(info)
        })?;

        Ok(Some(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr,
                msg: to_binary(&double_sided_liq_msg)?,
                funds: vec![native_coin, ls_coin],
            }),
            DOUBLE_SIDED_REPLY_ID,
        )))
    }
}

fn try_get_single_side_lp_submsg(
    deps: DepsMut,
    lp_contract: String,
) -> Result<Option<SubMsg>, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let asset_data = ASSETS.load(deps.storage)?;
    let single_side_lp_limits = SINGLE_SIDED_LP_LIMITS.load(deps.storage)?;

    let bal_coins = deps.querier.query_all_balances(lp_contract)?;

    // First we filter out non-relevant token balances
    let (native_bal, ls_bal) = get_relevant_balances(
        bal_coins,
        asset_data.clone().ls_asset_denom,
        asset_data.clone().native_asset_denom,
    );

    println!("native bal\t: {:?}", native_bal);
    println!("ls bal\t\t: {:?}", ls_bal);

    let native_asset = Asset {
        info: asset_data.get_native_asset_info(),
        amount: native_bal.amount,
    };
    let ls_asset = Asset {
        info: asset_data.get_ls_asset_info(),
        amount: ls_bal.amount,
    };

    // if both balances are non-zero we should provide double sided liquidity
    // if both balances are zero, we can't provide anything
    // in both cases we exit
    if (!native_bal.amount.is_zero() && !ls_bal.amount.is_zero())
        || (native_bal.amount.is_zero() && ls_bal.amount.is_zero())
    {
        return Ok(None);
    }

    let holder_address = HOLDER_ADDRESS.load(deps.storage)?;

    // given one non-zero asset, we build the ProvideLiquidity message
    let single_sided_liq_msg = ProvideLiquidity {
        assets: vec![ls_asset, native_asset],
        slippage_tolerance,
        auto_stake,
        receiver: Some(holder_address),
    };

    println!("single side liquidity msg: {:?}", single_sided_liq_msg);

    // now we try to submit the message for either LS or native single side liquidity
    if native_bal.amount.is_zero() && ls_bal.amount <= single_side_lp_limits.ls_asset_limit {
        // if available ls token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr,
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![ls_bal.clone()],
            }),
            SINGLE_SIDED_REPLY_ID,
        );
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_ls = info.provided_amount_ls.checked_add(ls_bal.amount)?;
            Ok(info)
        })?;
        return Ok(Some(submsg));
    } else if ls_bal.amount.is_zero()
        && native_bal.amount <= single_side_lp_limits.native_asset_limit
    {
        // if available native token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pool_address.addr,
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![native_bal.clone()],
            }),
            SINGLE_SIDED_REPLY_ID,
        );
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_native =
                info.provided_amount_native.checked_add(native_bal.amount)?;
            Ok(info)
        })?;
        return Ok(Some(submsg));
    }

    // if neither ls or native token single side lp message was built, we just go back and wait
    Ok(None)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::LpPosition {} => Ok(to_binary(&LP_POSITION.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::Assets {} => Ok(to_binary(&ASSETS.may_load(deps.storage)?)?),
        QueryMsg::ExpectedReturnAmount {} => Ok(to_binary(&EXPECTED_RETURN_AMOUNT.may_load(deps.storage)?)?),
        QueryMsg::AllowedReturnDelta {} => Ok(to_binary(&ALLOWED_RETURN_DELTA.may_load(deps.storage)?)?),
        QueryMsg::ExpectedNativeTokenAmount {} => Ok(to_binary(&EXPECTED_NATIVE_TOKEN_AMOUNT.may_load(deps.storage)?)?),
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
            price_delta,
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


            Ok(Response::default().add_attribute("method", "update_config"))
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    println!("{:?}", msg.clone().result.unwrap());
    match msg.id {
        DOUBLE_SIDED_REPLY_ID => handle_double_sided_reply_id(deps, _env, msg),
        SINGLE_SIDED_REPLY_ID => handle_single_sided_reply_id(deps, _env, msg),
        _ => Err(ContractError::from(StdError::GenericErr {
            msg: "err".to_string(),
        })),
    }
}

fn handle_double_sided_reply_id(
    _deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    // TODO: query balances here and if both are 0, exit?

    Ok(Response::default()
        .add_attribute("method", "handle_double_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string()))
}

fn handle_single_sided_reply_id(
    _deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    // TODO: query balances here and if both are 0, exit?
    Ok(Response::default()
        .add_attribute("method", "handle_single_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string()))
}
