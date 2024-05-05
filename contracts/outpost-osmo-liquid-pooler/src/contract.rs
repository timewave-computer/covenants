use std::str::FromStr;

use crate::{
    error::ContractError,
    msg::{
        CallerContext, ExecuteMsg, InstantiateMsg, MigrateMsg, OsmosisPool,
        OutpostProvideLiquidityConfig, OutpostWithdrawLiquidityConfig, QueryMsg,
    },
    state::PENDING_REPLY,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_string, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Fraction, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::must_pay;
use osmosis_std::{
    shim::Any,
    types::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        osmosis::gamm::v1beta1::{
            MsgExitPool, MsgJoinPool, MsgJoinSwapExternAmountIn, Pool,
            QueryCalcExitPoolCoinsFromSharesRequest, QueryCalcExitPoolCoinsFromSharesResponse,
            QueryCalcJoinPoolNoSwapSharesRequest, QueryCalcJoinPoolNoSwapSharesResponse,
            QueryCalcJoinPoolSharesRequest, QueryCalcJoinPoolSharesResponse, QueryPoolRequest,
            QueryPoolResponse,
        },
    },
};
use semver::Version;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const OSMO_POOL_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default().add_attribute("outpost", env.contract.address.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ProvideLiquidity { config } => try_provide_liquidity(deps, env, info, config),
        ExecuteMsg::WithdrawLiquidity { config } => try_withdraw_liquidity(deps, env, info, config),
    }
}

fn try_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: OutpostWithdrawLiquidityConfig,
) -> Result<Response, ContractError> {
    // first we query the pool for validation and info
    let query_response: QueryPoolResponse = deps.querier.query(
        &QueryPoolRequest {
            pool_id: config.pool_id.u64(),
        }
        .into(),
    )?;
    let osmo_pool: Pool = decode_osmo_pool_binary(query_response.pool)?;

    let pool_shares_coin = match osmo_pool.total_shares {
        Some(coin) => coin,
        None => {
            return Err(ContractError::OsmosisPoolError(
                "no shares coin in pool".to_string(),
            ))
        }
    };

    // we assert that the correct lp token is being redeemed
    let shares_to_redeem = must_pay(&info, &pool_shares_coin.denom)?;

    // we now estimate the underlying assets from those shares
    let calc_exit_query_response: QueryCalcExitPoolCoinsFromSharesResponse = deps.querier.query(
        &QueryCalcExitPoolCoinsFromSharesRequest {
            pool_id: config.pool_id.u64(),
            share_in_amount: shares_to_redeem.to_string(),
        }
        .into(),
    )?;

    // ensure that two assets are to be expected
    ensure!(
        calc_exit_query_response.tokens_out.len() == 2,
        ContractError::OsmosisPoolError("exit pool simulation must return 2 denoms".to_string())
    );

    // build the exit pool request based on the exit pool simulation
    let exit_pool_request: CosmosMsg = MsgExitPool {
        sender: env.contract.address.to_string(),
        pool_id: config.pool_id.u64(),
        share_in_amount: shares_to_redeem.to_string(),
        token_out_mins: calc_exit_query_response.tokens_out.clone(),
    }
    .into();

    // we build a context helper that will be used to
    // return the resulting funds (and/or leftovers) to the sender
    let callback_context = CallerContext {
        sender: info.sender.to_string(),
        gamm_denom: pool_shares_coin.denom.to_string(),
        pool_denom_1: calc_exit_query_response.tokens_out[0].denom.to_string(),
        pool_denom_2: calc_exit_query_response.tokens_out[1].denom.to_string(),
    };

    // store the callback context to be loaded in the callback
    PENDING_REPLY.save(deps.storage, &callback_context)?;

    Ok(Response::default()
        .add_attribute("method", "try_withdraw_liquidity")
        .add_submessage(SubMsg::reply_always(exit_pool_request, OSMO_POOL_REPLY_ID)))
}

fn try_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: OutpostProvideLiquidityConfig,
) -> Result<Response, ContractError> {
    ensure!(
        config.slippage_tolerance < Decimal::one(),
        ContractError::SlippageError {}
    );
    // first we query the pool for validation and info
    let query_response: QueryPoolResponse = deps.querier.query(
        &QueryPoolRequest {
            pool_id: config.pool_id.u64(),
        }
        .into(),
    )?;
    let osmo_pool: Pool = decode_osmo_pool_binary(query_response.pool)?;

    // validate that the pool we wish to provide liquidity
    // to is composed of two assets
    osmo_pool.validate_pool_assets_length()?;

    // only gamm 50:50 pools are supported (for now)
    osmo_pool.validate_pool_asset_weights()?;

    // collect the pool assets into cw coins
    let pool_assets = osmo_pool.get_pool_cw_coins()?;
    // get the total gamm shares cw_std coin
    let gamm_shares_coin = osmo_pool.get_gamm_cw_coin()?;

    // validate the price against our expectations
    let pool_spot_price = Decimal::from_ratio(pool_assets[0].amount, pool_assets[1].amount);
    let min_acceptable_spot_price = config.expected_spot_price - config.acceptable_price_spread;
    let max_acceptable_spot_price = config.expected_spot_price + config.acceptable_price_spread;

    if min_acceptable_spot_price > pool_spot_price || max_acceptable_spot_price < pool_spot_price {
        return Err(ContractError::PriceRangeError {});
    }

    // get the amounts paid of pool denoms
    let asset_1_received = Coin {
        denom: pool_assets[0].denom.to_string(),
        amount: get_paid_denom_amount(&info, &pool_assets[0].denom).unwrap_or(Uint128::zero()),
    };
    let asset_2_received = Coin {
        denom: pool_assets[1].denom.to_string(),
        amount: get_paid_denom_amount(&info, &pool_assets[1].denom).unwrap_or(Uint128::zero()),
    };

    // we build a context helper that will be used to
    // return the resulting funds to the sender
    let callback_context = CallerContext {
        sender: info.sender.to_string(),
        gamm_denom: gamm_shares_coin.denom.to_string(),
        pool_denom_1: asset_1_received.denom.to_string(),
        pool_denom_2: asset_2_received.denom.to_string(),
    };

    // depending on which assets we have available,
    // we construct different liquidity provision message
    match (
        !asset_1_received.amount.is_zero(),
        !asset_2_received.amount.is_zero(),
    ) {
        // both assets provided, attempt to provide two sided liquidity
        (true, true) => provide_double_sided_liquidity(
            deps,
            env,
            osmo_pool,
            vec![asset_1_received, asset_2_received],
            config.slippage_tolerance,
            callback_context,
        ),
        // only asset 1 is provided, attempt to provide single sided
        (true, false) => provide_single_sided_liquidity(
            deps,
            osmo_pool,
            asset_1_received,
            env.contract.address.to_string(),
            config.slippage_tolerance,
            config.asset_1_single_side_lp_limit,
            callback_context,
        ),
        // only asset 2 is provided, attempt to provide single sided
        (false, true) => provide_single_sided_liquidity(
            deps,
            osmo_pool,
            asset_2_received,
            env.contract.address.to_string(),
            config.slippage_tolerance,
            config.asset_2_single_side_lp_limit,
            callback_context,
        ),
        // no funds provided, error out
        (false, false) => Err(ContractError::LiquidityProvisionError(
            "no funds provided".to_string(),
        )),
    }
}

fn provide_double_sided_liquidity(
    deps: DepsMut,
    env: Env,
    pool: Pool,
    assets_paid: Vec<Coin>,
    slippage_tolerance: Decimal,
    callback_ctx: CallerContext,
) -> Result<Response, ContractError> {
    let token_in_maxs: Vec<ProtoCoin> =
        vec![assets_paid[0].clone().into(), assets_paid[1].clone().into()];

    // first we query the expected gamm amount
    let query_response: QueryCalcJoinPoolNoSwapSharesResponse = deps.querier.query(
        &QueryCalcJoinPoolNoSwapSharesRequest {
            pool_id: pool.id,
            tokens_in: token_in_maxs.clone(),
        }
        .into(),
    )?;

    // expected gamm tokens
    let response_gamm_coin = Coin {
        denom: callback_ctx.gamm_denom.to_string(),
        amount: Uint128::from_str(&query_response.shares_out)?,
    };
    let expected_gamm_coin = apply_slippage(slippage_tolerance, response_gamm_coin)?;

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: env.contract.address.to_string(),
        pool_id: pool.id,
        // exact number of shares we wish to receive
        share_out_amount: expected_gamm_coin.amount.to_string(),
        token_in_maxs,
    }
    .into();

    // store the callback context to be loaded in the callback
    PENDING_REPLY.save(deps.storage, &callback_ctx)?;

    Ok(Response::default()
        .add_attribute("method", "try_join_pool")
        .add_submessage(SubMsg::reply_always(osmo_msg, OSMO_POOL_REPLY_ID)))
}

fn provide_single_sided_liquidity(
    deps: DepsMut,
    pool: Pool,
    asset_paid: Coin,
    outpost: String,
    slippage_tolerance: Decimal,
    single_side_limit: Uint128,
    callback_ctx: CallerContext,
) -> Result<Response, ContractError> {
    ensure!(
        asset_paid.amount <= single_side_limit,
        ContractError::SingleSideLiquidityProvisionError(
            single_side_limit.to_string(),
            asset_paid.amount.to_string(),
        )
    );
    // first we query the expected gamm amount
    let query_response: QueryCalcJoinPoolSharesResponse = deps.querier.query(
        &QueryCalcJoinPoolSharesRequest {
            pool_id: pool.id,
            tokens_in: vec![asset_paid.clone().into()],
        }
        .into(),
    )?;

    let response_gamm_coin = Coin {
        denom: callback_ctx.gamm_denom.to_string(),
        amount: Uint128::from_str(&query_response.share_out_amount)?,
    };
    let expected_gamm_coin = apply_slippage(slippage_tolerance, response_gamm_coin)?;

    let join_pool_msg: CosmosMsg = MsgJoinSwapExternAmountIn {
        sender: outpost,
        pool_id: pool.id,
        token_in: Some(asset_paid.clone().into()),
        share_out_min_amount: expected_gamm_coin.amount.to_string(),
    }
    .into();

    // store the callback context to be loaded in the callback
    PENDING_REPLY.save(deps.storage, &callback_ctx)?;

    Ok(Response::default()
        .add_attribute("method", "try_join_pool")
        .add_submessage(SubMsg::reply_always(join_pool_msg, OSMO_POOL_REPLY_ID)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::NotFound {
        kind: "not implemented".to_string(),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        OSMO_POOL_REPLY_ID => handle_pool_interaction_reply(deps, env),
        _ => Err(ContractError::UnknownReplyId(msg.id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::UpdateCodeId { data: _ } => {
            let version: Version = match CONTRACT_VERSION.parse() {
                Ok(v) => v,
                Err(e) => return Err(StdError::generic_err(e.to_string())),
            };

            let storage_version: Version = match get_contract_version(deps.storage)?.version.parse()
            {
                Ok(v) => v,
                Err(e) => return Err(StdError::generic_err(e.to_string())),
            };
            if storage_version < version {
                set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            }
            Ok(Response::new())
        }
    }
}

fn handle_pool_interaction_reply(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // load and clear the pending reply that we are processing
    let callback_ctx = PENDING_REPLY.load(deps.storage)?;
    PENDING_REPLY.remove(deps.storage);

    // we query the balances of relevant denoms
    let available_gamm = deps.querier.query_balance(
        env.contract.address.to_string(),
        callback_ctx.gamm_denom.to_string(),
    )?;
    let leftover_asset_1 = deps
        .querier
        .query_balance(env.contract.address.to_string(), callback_ctx.pool_denom_1)?;
    let leftover_asset_2 = deps
        .querier
        .query_balance(env.contract.address.to_string(), callback_ctx.pool_denom_2)?;

    // and collect them into tokens to be refunded (if any)
    let refund_tokens: Vec<Coin> = vec![available_gamm, leftover_asset_1, leftover_asset_2]
        .into_iter()
        .filter(|c| c.amount > Uint128::zero())
        .collect();

    let mut response = Response::default().add_attribute("method", "handle_pool_interaction_reply");

    if !refund_tokens.is_empty() {
        response = response.add_message(BankMsg::Send {
            to_address: callback_ctx.sender,
            amount: refund_tokens.clone(),
        });
    }

    Ok(response.add_attribute("refund_tokens", to_json_string(&refund_tokens)?))
}

/// cw-utils must pay requires specifically one coin, this is a helper
/// for multi-coin inputs
fn get_paid_denom_amount(info: &MessageInfo, target_denom: &str) -> StdResult<Uint128> {
    for coin in &info.funds {
        if coin.denom == target_denom {
            return Ok(coin.amount);
        }
    }
    Err(StdError::not_found(target_denom))
}

fn decode_osmo_pool_binary(pool: Option<Any>) -> StdResult<Pool> {
    let osmo_shim = match pool {
        Some(shim) => shim,
        None => {
            return Err(StdError::NotFound {
                kind: "shim not found".to_string(),
            })
        }
    };

    match osmo_shim.try_into() {
        Ok(result) => Ok(result),
        Err(err) => Err(StdError::InvalidBase64 {
            msg: err.to_string(),
        }),
    }
}

fn apply_slippage(slippage: Decimal, coin: Coin) -> Result<Coin, ContractError> {
    let applied_slippage_amount = match coin
        .amount
        .checked_multiply_ratio(slippage.numerator(), slippage.denominator())
    {
        Ok(val) => val,
        Err(e) => return Err(StdError::generic_err(e.to_string()).into()),
    };

    Ok(Coin {
        denom: coin.denom,
        amount: coin.amount - applied_slippage_amount,
    })
}
