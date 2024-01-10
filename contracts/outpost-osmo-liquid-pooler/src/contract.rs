use std::str::FromStr;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, OsmosisPool, QueryMsg},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Fraction, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use osmosis_std::{
    shim::Any,
    types::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        osmosis::gamm::v1beta1::{
            MsgJoinPool, MsgJoinSwapExternAmountIn, Pool, QueryCalcJoinPoolSharesRequest,
            QueryCalcJoinPoolSharesResponse, QueryPoolRequest, QueryPoolResponse,
        },
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-outpost-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        ExecuteMsg::ProvideLiquidity {
            pool_id,
            expected_spot_price,
            acceptable_price_spread,
            slippage_tolerance,
            asset_1_single_side_lp_limit,
            asset_2_single_side_lp_limit,
        } => {
            ensure!(
                slippage_tolerance < Decimal::one(),
                ContractError::SlippageError {}
            );
            // first we query the pool for validation and info
            let query_response: QueryPoolResponse = deps.querier.query(
                &QueryPoolRequest { pool_id: pool_id.u64() }.into(),
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
            let min_acceptable_spot_price = expected_spot_price - acceptable_price_spread;
            let max_acceptable_spot_price = expected_spot_price + acceptable_price_spread;

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
                    osmo_pool,
                    vec![asset_1_received, asset_2_received],
                    pool_assets,
                    info.sender.to_string(),
                    env.contract.address.to_string(),
                    gamm_shares_coin,
                    slippage_tolerance,
                ),
                // only asset 1 is provided, attempt to provide single sided
                (true, false) => provide_single_sided_liquidity(
                    deps,
                    osmo_pool,
                    asset_1_received,
                    env.contract.address.to_string(),
                    info.sender.to_string(),
                    gamm_shares_coin,
                    slippage_tolerance,
                    asset_1_single_side_lp_limit,
                ),
                // only asset 2 is provided, attempt to provide single sided
                (false, true) => provide_single_sided_liquidity(
                    deps,
                    osmo_pool,
                    asset_2_received,
                    env.contract.address.to_string(),
                    info.sender.to_string(),
                    gamm_shares_coin,
                    slippage_tolerance,
                    asset_2_single_side_lp_limit,
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
    pool: Pool,
    assets_paid: Vec<Coin>,
    pool_assets: Vec<Coin>,
    sender: String,
    outpost: String,
    gamm_coin: Coin,
    slippage_tolerance: Decimal,
) -> Result<Response, ContractError> {
    // we compare the assets that were paid against the pool balances
    // to derive the amount of gamm tokens we can expect
    let expected_gamm_shares = std::cmp::min(
        assets_paid[0]
            .amount
            .multiply_ratio(gamm_coin.amount, pool_assets[0].amount),
        assets_paid[1]
            .amount
            .multiply_ratio(gamm_coin.amount, pool_assets[1].amount),
    );

    // use that amount to get a cw coin
    let expected_coin =  Coin { denom: gamm_coin.denom.to_string(), amount: expected_gamm_shares };

    // apply the slippage
    let expected_gamm_slippaged = apply_slippage(slippage_tolerance, expected_coin)?;

    let token_in_maxs: Vec<ProtoCoin> =
        vec![assets_paid[0].clone().into(), assets_paid[1].clone().into()];

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: outpost,
        pool_id: pool.id,
        // exact number of shares we wish to receive
        share_out_amount: expected_gamm_slippaged.amount.to_string(),
        token_in_maxs,
    }
    .into();

    let gamm_transfer: CosmosMsg = BankMsg::Send {
        to_address: sender,
        amount: vec![expected_gamm_slippaged],
    }
    .into();

    Ok(Response::default()
        .add_messages(vec![osmo_msg, gamm_transfer])
        .add_attribute("method", "provide_double_sided_liquidity")
        .add_attribute("pool", to_json_binary(&pool)?.to_string())
        .add_attribute("asset_1_paid", to_json_binary(&assets_paid[0])?.to_string())
        .add_attribute("asset_2_paid", to_json_binary(&assets_paid[1])?.to_string()))
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

    let expected_gamm_shares = Uint128::from_str(&query_response.share_out_amount)?;
    let response_gamm_coin = Coin {
        denom: gamm_coin.denom,
        amount: expected_gamm_shares,
    };
    let expected_gamm_coin = apply_slippage(slippage_tolerance, response_gamm_coin)?;

    let join_pool_msg = MsgJoinSwapExternAmountIn {
        sender: outpost,
        pool_id: pool.id,
        token_in: Some(asset_paid.clone().into()),
        share_out_min_amount: expected_gamm_coin.amount.to_string(),
    };

    let gamm_transfer: CosmosMsg = BankMsg::Send {
        to_address: sender,
        amount: vec![expected_gamm_coin],
    }
    .into();

    Ok(Response::default()
        .add_messages(vec![join_pool_msg.into(), gamm_transfer])
        .add_attribute("method", "provide_single_sided_liquidity")
        .add_attribute("pool", to_json_binary(&pool)?.to_string())
        .add_attribute("asset_paid", to_json_binary(&asset_paid)?.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(cosmwasm_std::StdError::NotFound {
        kind: "not implemented".to_string(),
    })
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
