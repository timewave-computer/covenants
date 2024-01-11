use std::str::FromStr;

use crate::{
    error::ContractError,
    msg::{InstantiateMsg, OsmosisPool, QueryMsg, JoinPoolMsgContext, ExecuteMsg}, state::{PENDING_REPLY},
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Fraction, MessageInfo, Response, StdError, StdResult, Uint128, Reply, SubMsg, SubMsgResult, from_json, to_json_string, Order,
};
use cw2::set_contract_version;
use osmosis_std::{
    shim::Any,
    types::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        osmosis::gamm::v1beta1::{
            MsgJoinPool, MsgJoinSwapExternAmountIn, Pool, QueryCalcJoinPoolSharesRequest,
            QueryCalcJoinPoolSharesResponse, QueryPoolRequest, QueryPoolResponse, QueryCalcJoinPoolNoSwapSharesRequest, QueryCalcJoinPoolNoSwapSharesResponse,
        },
    },
};


const CONTRACT_NAME: &str = "crates.io:covenant-outpost-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROVIDE_LIQUIDITY_REPLY_ID: u64 = 1;

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
        ExecuteMsg::ProvideLiquidity { config } => {
            ensure!(
                config.slippage_tolerance < Decimal::one(),
                ContractError::SlippageError {}
            );
            // first we query the pool for validation and info
            let query_response: QueryPoolResponse = deps.querier.query(
                &QueryPoolRequest { pool_id: config.pool_id.u64() }.into(),
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

            if min_acceptable_spot_price > pool_spot_price || max_acceptable_spot_price < pool_spot_price
            {
                return Err(ContractError::PriceRangeError {});
            }

            // get the amounts paid of pool denoms
            let asset_1_received = Coin {
                denom: pool_assets[0].denom.to_string(),
                amount: get_paid_denom_amount(&info, &pool_assets[0].denom)
                    .unwrap_or(Uint128::zero()),
            };
            let asset_2_received = Coin {
                denom: pool_assets[1].denom.to_string(),
                amount: get_paid_denom_amount(&info, &pool_assets[1].denom)
                    .unwrap_or(Uint128::zero()),
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
                    info.sender.to_string(),
                    gamm_shares_coin,
                    config.slippage_tolerance,
                ),
                // only asset 1 is provided, attempt to provide single sided
                (true, false) => provide_single_sided_liquidity(
                    deps,
                    osmo_pool,
                    asset_1_received,
                    env.contract.address.to_string(),
                    info.sender.to_string(),
                    gamm_shares_coin,
                    config.slippage_tolerance,
                    config.asset_1_single_side_lp_limit,
                ),
                // only asset 2 is provided, attempt to provide single sided
                (false, true) => provide_single_sided_liquidity(
                    deps,
                    osmo_pool,
                    asset_2_received,
                    env.contract.address.to_string(),
                    info.sender.to_string(),
                    gamm_shares_coin,
                    config.slippage_tolerance,
                    config.asset_2_single_side_lp_limit,
                ),
                // no funds provided, error out
                (false, false) => Err(ContractError::LiquidityProvisionError(
                    "no funds provided".to_string(),
                )),
            }
        }
    }
}

fn provide_double_sided_liquidity(
    deps: DepsMut,
    env: Env,
    pool: Pool,
    assets_paid: Vec<Coin>,
    sender: String,
    gamm_coin: Coin,
    slippage_tolerance: Decimal,
) -> Result<Response, ContractError> {
    let token_in_maxs: Vec<ProtoCoin> = vec![
        assets_paid[0].clone().into(),
        assets_paid[1].clone().into(),
    ];

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
        denom: gamm_coin.denom.to_string(),
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

    // we build a context helper that will be used to
    // return the resulting funds to the sender
    let callback_context = JoinPoolMsgContext {
        sender,
        assets_paid,
        gamm_denom: gamm_coin.denom,
    };

    // store the callback context to be loaded in the callback
    PENDING_REPLY.save(deps.storage, &callback_context)?;

    Ok(Response::default()
        .add_attribute("method", "try_join_pool")
        .add_submessage(SubMsg::reply_always(osmo_msg, PROVIDE_LIQUIDITY_REPLY_ID))
        .set_data(to_json_binary(&callback_context)?)
    )
}

fn handle_provide_liquidity_reply(
    deps: DepsMut, env: Env, msg: Reply, callback_context: JoinPoolMsgContext,
) -> Result<Response, ContractError> {

    let mut refund_tokens = vec![];

    let available_gamm = deps.querier.query_balance(
        env.contract.address.to_string(),
        callback_context.gamm_denom.to_string(),
    )?;

    if available_gamm.amount > Uint128::zero() {
        refund_tokens.push(available_gamm.clone());
    }
    // we iterate over assets that were sent to this contract
    // and include all available assets after providing liquidity
    for asset_paid in callback_context.clone().assets_paid {
        let leftover_asset = deps.querier.query_balance(
            env.contract.address.to_string(),
            asset_paid.denom.to_string(),
        )?;
        if leftover_asset.amount > Uint128::zero() {
            refund_tokens.push(leftover_asset);
        }
    }

    let mut response = Response::default()
        .add_attribute("method", "handle_provide_liquidity_reply")
        .add_attribute("refund_tokens", to_json_string(&refund_tokens)?);

    response = match msg.result {
        SubMsgResult::Ok(r) => {
            if let Some(submsg_response_data) = r.clone().data {
                response = response.add_attribute("submsg_response_data", submsg_response_data.to_string())
            }
            response.add_attribute("submsg_response", to_json_string(&r)?)
        },
        SubMsgResult::Err(e) => response.add_attribute("submsg_error", e),
    };

    if !refund_tokens.is_empty() {
        response = response.add_message(BankMsg::Send {
            to_address: callback_context.sender,
            amount: refund_tokens,
        });
    }

    PENDING_REPLY.remove(deps.storage);

    Ok(response)
}

fn provide_single_sided_liquidity(
    deps: DepsMut,
    pool: Pool,
    asset_paid: Coin,
    outpost: String,
    sender: String,
    gamm_coin: Coin,
    slippage_tolerance: Decimal,
    single_side_limit: Uint128,
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
        denom: gamm_coin.denom,
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

    let callback_context = JoinPoolMsgContext {
        sender,
        assets_paid: vec![asset_paid],
        gamm_denom: expected_gamm_coin.denom,
    };

    Ok(Response::default()
        .add_attribute("method", "try_join_pool")
        .set_data(to_json_binary(&callback_context)?)
        .add_submessage(SubMsg::reply_always(join_pool_msg, PROVIDE_LIQUIDITY_REPLY_ID))
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(cosmwasm_std::StdError::NotFound {
        kind: "not implemented".to_string(),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    // load reply here
    let pending_reply = PENDING_REPLY.load(deps.storage)?;
    match msg.id {
        PROVIDE_LIQUIDITY_REPLY_ID => handle_provide_liquidity_reply(deps, env, msg, pending_reply),
        _ => {
            // Err(ContractError::from(StdError::generic_err(format!("unknown msg reply id: {:?}", msg.id))))
            Ok(Response::default()
                .add_attribute("status", "unknown_reply_id".to_string())
                .add_attribute("reply_id", msg.id.to_string())
            )
        },
    }
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
