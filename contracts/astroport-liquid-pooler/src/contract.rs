#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use covenant_clock::helpers::{enqueue_msg, verify_clock};
use covenant_utils::{astroport::query_astro_pool_token, withdraw_lp_helper::WithdrawLPMsgs};
use cw2::set_contract_version;

use astroport::{
    asset::{Asset, PairInfo},
    pair::{Cw20HookMsg, ExecuteMsg::ProvideLiquidity, PoolResponse},
    DecimalCheckedOps,
};
use cw20::Cw20ExecuteMsg;
use cw_utils::parse_reply_execute_data;

use crate::{
    error::ContractError,
    msg::{
        ContractState, DecimalRange, ExecuteMsg, InstantiateMsg, LpConfig, MigrateMsg,
        ProvidedLiquidityInfo, QueryMsg,
    },
    state::{HOLDER_ADDRESS, LP_CONFIG, PROVIDED_LIQUIDITY_INFO},
};

use neutron_sdk::NeutronResult;

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-astroport-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DOUBLE_SIDED_REPLY_ID: u64 = 321u64;
const SINGLE_SIDED_REPLY_ID: u64 = 322u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate the contract addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;

    // validate that the pool did not migrate to a new pair type
    let pool_response: PairInfo = deps
        .querier
        .query_wasm_smart(pool_addr.to_string(), &astroport::pair::QueryMsg::Pair {})?;

    ensure!(
        pool_response.pair_type.eq(&msg.pair_type),
        ContractError::PairTypeMismatch {}
    );

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant module addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;

    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;

    let decimal_range = DecimalRange::try_from(
        msg.pool_price_config.expected_spot_price,
        msg.pool_price_config.acceptable_price_spread,
    )?;

    let lp_config = LpConfig {
        pool_address: pool_addr,
        single_side_lp_limits: msg.single_side_lp_limits,
        slippage_tolerance: msg.slippage_tolerance,
        expected_pool_ratio_range: decimal_range,
        pair_type: msg.pair_type,
        asset_data: msg.assets,
    };
    LP_CONFIG.save(deps.storage, &lp_config)?;

    // we begin with no liquidity provided
    PROVIDED_LIQUIDITY_INFO.save(
        deps.storage,
        &ProvidedLiquidityInfo {
            provided_amount_a: Uint128::zero(),
            provided_amount_b: Uint128::zero(),
        },
    )?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "lp_instantiate")
        .add_attribute("clock_addr", clock_addr)
        .add_attributes(lp_config.to_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Withdraw { percentage } => try_withdraw(deps, env, info, percentage),
    }
}

fn try_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    percent: Option<Decimal>,
) -> Result<Response, ContractError> {
    let percent = percent.unwrap_or(Decimal::one());
    ensure!(
        percent > Decimal::zero() && percent <= Decimal::one(),
        ContractError::WithdrawPercentageRangeError {}
    );

    let holder_addr = HOLDER_ADDRESS.load(deps.storage)?;
    ensure!(info.sender == holder_addr, ContractError::NotHolder {});

    // Query LP position of the LPer
    let lp_config = LP_CONFIG.load(deps.storage)?;
    let lp_token_info = query_astro_pool_token(
        deps.querier,
        lp_config.pool_address.to_string(),
        env.contract.address.to_string(),
    )?;

    // if no lp tokens are available, we attempt to withdraw any available denoms
    if lp_token_info.balance_response.balance.is_zero() {
        let asset_a_bal = deps.querier.query_balance(
            env.contract.address.to_string(),
            lp_config.asset_data.asset_a_denom.as_str(),
        )?;
        let asset_b_bal = deps.querier.query_balance(
            env.contract.address.to_string(),
            lp_config.asset_data.asset_b_denom.as_str(),
        )?;

        let mut funds = vec![];

        if !asset_a_bal.amount.is_zero() {
            funds.push(asset_a_bal);
        }

        if !asset_b_bal.amount.is_zero() {
            funds.push(asset_b_bal);
        }

        ensure!(!funds.is_empty(), ContractError::NothingToWithdraw {});

        return Ok(Response::default().add_message(WasmMsg::Execute {
            contract_addr: holder_addr.to_string(),
            msg: to_json_binary(&WithdrawLPMsgs::Distribute {})?,
            funds,
        }));
    }

    // If percentage is 100%, use the whole balance
    // If percentage is less than 100%, calculate the percentage of share we want to withdraw
    let withdraw_shares_amount = if percent == Decimal::one() {
        lp_token_info.balance_response.balance
    } else {
        Decimal::from_atomics(lp_token_info.balance_response.balance, 0)?
            .checked_mul(percent)?
            .to_uint_floor()
    };

    // Clculate the withdrawn amount of A and B tokens from the shares we have
    let withdrawn_coins = deps
        .querier
        .query_wasm_smart::<Vec<Asset>>(
            lp_config.pool_address.to_string(),
            &astroport::pair::QueryMsg::Share {
                amount: withdraw_shares_amount,
            },
        )?
        .iter()
        .map(|asset| asset.to_coin())
        .collect::<Result<Vec<Coin>, _>>()?;

    // exit pool and withdraw funds with the shares calculated
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = WasmMsg::Execute {
        contract_addr: lp_token_info.pair_info.liquidity_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: lp_config.pool_address.to_string(),
            amount: withdraw_shares_amount,
            msg: to_json_binary(withdraw_liquidity_hook)?,
        })?,
        funds: vec![],
    };

    // send message to holder that we finished with the withdrawal
    // with the funds we withdrew from the pool
    let to_holder_msg = WasmMsg::Execute {
        contract_addr: holder_addr.to_string(),
        msg: to_json_binary(&WithdrawLPMsgs::Distribute {})?,
        funds: withdrawn_coins,
    };

    Ok(Response::default()
        .add_message(withdraw_msg)
        .add_message(to_holder_msg))
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_lp(deps, env),
    }
}

/// method which attempts to provision liquidity to the pool.
/// if both desired asset balances are non-zero, double sided liquidity
/// is provided.
/// otherwise, single-sided liquidity provision is attempted.
fn try_lp(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let lp_config = LP_CONFIG.load(deps.storage)?;

    let pool_response: PoolResponse = deps
        .querier
        .query_wasm_smart(&lp_config.pool_address, &astroport::pair::QueryMsg::Pool {})?;

    let (pool_token_a_bal, pool_token_b_bal) = get_pool_asset_amounts(
        pool_response.assets,
        lp_config.asset_data.asset_a_denom.as_str(),
        lp_config.asset_data.asset_b_denom.as_str(),
    )?;
    let a_to_b_ratio = Decimal::from_ratio(pool_token_a_bal, pool_token_b_bal);
    // validate the current pool ratio against our expectations
    lp_config
        .expected_pool_ratio_range
        .is_within_range(a_to_b_ratio)?;

    // first we query our own balances
    let coin_a = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.asset_data.asset_a_denom.as_str(),
    )?;
    let coin_b = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.asset_data.asset_b_denom.as_str(),
    )?;

    // depending on available balances we attempt a different action:
    match (coin_a.amount.is_zero(), coin_b.amount.is_zero()) {
        // exactly one balance is non-zero, we attempt single-side
        (true, false) | (false, true) => {
            let single_sided_submsg =
                try_get_single_side_lp_submsg(deps.branch(), env, (coin_a, coin_b), lp_config)?;
            if let Some(msg) = single_sided_submsg {
                return Ok(Response::default()
                    .add_submessage(msg)
                    .add_attribute("method", "single_side_lp"));
            }
        }
        // both balances are non-zero, we attempt double-side
        (false, false) => {
            let double_sided_submsg = try_get_double_side_lp_submsg(
                deps.branch(),
                env,
                (coin_a, coin_b),
                a_to_b_ratio,
                (pool_token_a_bal, pool_token_b_bal),
                lp_config,
            )?;
            if let Some(msg) = double_sided_submsg {
                return Ok(Response::default()
                    .add_submessage(msg)
                    .add_attribute("method", "double_side_lp"));
            }
        }
        // both balances zero, no liquidity can be provisioned
        _ => (),
    }

    // if no message could be constructed, we keep waiting for funds
    Ok(Response::default()
        .add_attribute("method", "try_lp")
        .add_attribute("status", "not enough funds"))
}

/// attempts to get a double sided ProvideLiquidity submessage.
/// amounts here do not matter. as long as we have non-zero balances of both
/// a and b tokens, the maximum amount of liquidity is provided to maintain
/// the existing pool ratio.
fn try_get_double_side_lp_submsg(
    deps: DepsMut,
    env: Env,
    (token_a, token_b): (Coin, Coin),
    pool_token_ratio: Decimal,
    (pool_token_a_bal, pool_token_b_bal): (Uint128, Uint128),
    lp_config: LpConfig,
) -> Result<Option<SubMsg>, ContractError> {
    // we thus find the required token amount to enter into the position using all available b tokens:
    let required_token_a_amount = pool_token_ratio.checked_mul_uint128(token_b.amount)?;

    // depending on available balances we determine the highest amount
    // of liquidity we can provide:
    let (asset_a_double_sided, asset_b_double_sided) = if token_a.amount >= required_token_a_amount
    {
        // if we are able to satisfy the required amount, we do that:
        // provide all b tokens along with required amount of a tokens
        lp_config
            .asset_data
            .to_tuple(required_token_a_amount, token_b.amount)
    } else {
        // otherwise, our token a amount is insufficient to provide double
        // sided liquidity using all of our b tokens.
        // this means that we should provide all of our available a tokens,
        // and as many b tokens as needed to satisfy the existing ratio
        let ratio = Decimal::from_ratio(pool_token_b_bal, pool_token_a_bal);
        lp_config
            .asset_data
            .to_tuple(token_a.amount, ratio.checked_mul_uint128(token_a.amount)?)
    };

    let a_coin = asset_a_double_sided.to_coin()?;
    let b_coin = asset_b_double_sided.to_coin()?;

    // craft a ProvideLiquidity message with the determined assets
    let double_sided_liq_msg = ProvideLiquidity {
        assets: vec![asset_a_double_sided, asset_b_double_sided],
        slippage_tolerance: lp_config.slippage_tolerance,
        auto_stake: Some(false),
        receiver: Some(env.contract.address.to_string()),
    };

    // update the provided amounts and leftover assets
    PROVIDED_LIQUIDITY_INFO.update(
        deps.storage,
        |mut info: ProvidedLiquidityInfo| -> StdResult<_> {
            info.provided_amount_b = info.provided_amount_b.checked_add(b_coin.amount)?;
            info.provided_amount_a = info.provided_amount_a.checked_add(a_coin.amount)?;
            Ok(info)
        },
    )?;

    Ok(Some(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: lp_config.pool_address.to_string(),
            msg: to_json_binary(&double_sided_liq_msg)?,
            funds: vec![a_coin, b_coin],
        }),
        DOUBLE_SIDED_REPLY_ID,
    )))
}

/// attempts to build a single sided `ProvideLiquidity` message.
/// pool ratio does not get validated here. as long as the single
/// side asset amount being provided is within our predefined
/// single-side liquidity limits, we provide it.
fn try_get_single_side_lp_submsg(
    deps: DepsMut,
    env: Env,
    (coin_a, coin_b): (Coin, Coin),
    lp_config: LpConfig,
) -> Result<Option<SubMsg>, ContractError> {
    let assets = lp_config
        .asset_data
        .to_asset_vec(coin_a.amount, coin_b.amount);

    // given one non-zero asset, we build the ProvideLiquidity message
    let single_sided_liq_msg = ProvideLiquidity {
        assets,
        slippage_tolerance: lp_config.slippage_tolerance,
        auto_stake: Some(false),
        receiver: Some(env.contract.address.to_string()),
    };

    // now we try to submit the message for either B or A token single side liquidity
    if coin_a.amount.is_zero() && coin_b.amount <= lp_config.single_side_lp_limits.asset_b_limit {
        // update the provided liquidity info
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_b = info.provided_amount_b.checked_add(coin_b.amount)?;
            Ok(info)
        })?;

        // if available ls token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: lp_config.pool_address.to_string(),
                msg: to_json_binary(&single_sided_liq_msg)?,
                funds: vec![coin_b],
            }),
            SINGLE_SIDED_REPLY_ID,
        );

        return Ok(Some(submsg));
    }

    if coin_b.amount.is_zero() && coin_a.amount <= lp_config.single_side_lp_limits.asset_a_limit {
        // update the provided liquidity info
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_a = info.provided_amount_a.checked_add(coin_a.amount)?;
            Ok(info)
        })?;

        // if available A token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: lp_config.pool_address.to_string(),
                msg: to_json_binary(&single_sided_liq_msg)?,
                funds: vec![coin_a],
            }),
            SINGLE_SIDED_REPLY_ID,
        );

        return Ok(Some(submsg));
    }

    // if neither a nor b token single side lp message was built, we just go back and wait
    Ok(None)
}

/// filters out irrelevant balances and returns a and b token amounts
fn get_pool_asset_amounts(
    assets: Vec<Asset>,
    a_denom: &str,
    b_denom: &str,
) -> Result<(Uint128, Uint128), StdError> {
    let (mut a_bal, mut b_bal) = (Uint128::zero(), Uint128::zero());

    for asset in assets {
        let coin = asset.to_coin()?;
        if coin.denom == b_denom {
            // found b balance
            b_bal = coin.amount;
        } else if coin.denom == a_denom {
            // found a token balance
            a_bal = coin.amount;
        }
    }

    Ok((a_bal, b_bal))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::LpConfig {} => Ok(to_json_binary(&LP_CONFIG.may_load(deps.storage)?)?),
        // the deposit address for LP module is the contract itself
        QueryMsg::DepositAddress {} => {
            Ok(to_json_binary(&Some(&env.contract.address.to_string()))?)
        }
        QueryMsg::ProvidedLiquidityInfo {} => Ok(to_json_binary(
            &PROVIDED_LIQUIDITY_INFO.load(deps.storage)?,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> NeutronResult<Response> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            holder_address,
            lp_config,
        } => {
            let mut response = Response::default().add_attribute("method", "update_config");

            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
                response = response.add_attribute("clock_addr", clock_addr);
            }

            if let Some(holder_address) = holder_address {
                HOLDER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&holder_address)?)?;
                response = response.add_attribute("holder_address", holder_address);
            }

            if let Some(config) = lp_config {
                // validate the address before storing it
                deps.api.addr_validate(config.pool_address.as_str())?;
                LP_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            Ok(response)
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
    match parse_reply_execute_data(msg) {
        Ok(response) => Ok(Response::default()
            .add_attribute("method", "handle_double_sided_reply_id")
            .add_attribute(
                "response",
                match response.data {
                    Some(val) => val.to_base64(),
                    None => "none".to_string(),
                },
            )),
        Err(err) => Ok(Response::default()
            .add_attribute("method", "handle_double_sided_reply_id")
            .add_attribute("error", err.to_string())),
    }
}

fn handle_single_sided_reply_id(
    _deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    match parse_reply_execute_data(msg) {
        Ok(response) => Ok(Response::default()
            .add_attribute("method", "handle_single_sided_reply_id")
            .add_attribute(
                "response",
                match response.data {
                    Some(val) => val.to_base64(),
                    None => "none".to_string(),
                },
            )),
        Err(err) => Ok(Response::default()
            .add_attribute("method", "handle_single_sided_reply_id")
            .add_attribute("error", err.to_string())),
    }
}
