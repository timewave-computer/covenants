use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, QueryRequest, Empty, StdError, Coin, Uint128, Decimal, CosmosMsg, BankMsg,
};
use cw2::set_contract_version;
use cw_utils::must_pay;
use osmosis_std::{types::{osmosis::gamm::v1beta1::{QueryPoolRequest, QueryPoolResponse, Pool, MsgJoinPool}, cosmos::base::v1beta1::Coin as ProtoCoin}, shim::Any};
use crate::{
    error::ContractError,
    msg::{
        ExecuteMsg, InstantiateMsg, QueryMsg,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-outpost-osmo-liquid-pooler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()
        .add_attribute("outpost", env.contract.address.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ProvideLiquidity { pool_id } => {
            // first we query the pool to validate it and deduct ratios
            let query_pool_request: QueryRequest<Empty> = QueryPoolRequest {
                pool_id: pool_id.u64(),
            }
            .into();

            let query_response: QueryPoolResponse = deps.querier.query(&query_pool_request)?;
            let osmo_pool: Pool = decode_osmo_pool_binary(query_response.pool)?;

            // validate that the pool we wish to provide liquidity
            // to is composed of two assets
            if osmo_pool.pool_assets.len() != 2 {
                return Err(ContractError::OsmosisPoolError("pool must have 2 assets".to_string()))
            }

            // only gamm 50:50 pools are supported (for now)
            if osmo_pool.pool_assets[0].weight != osmo_pool.pool_assets[1].weight {
                return Err(ContractError::PoolRatioError(
                    format!("{:?}:{:?}", osmo_pool.pool_assets[0].weight, osmo_pool.pool_assets[1].weight)
                ))
            }

            // collect the pool assets into cw coins
            let mut pool_assets: Vec<Coin> = vec![];
            for pool_asset in osmo_pool.clone().pool_assets {
                match pool_asset.token {
                    Some(t) => pool_assets.push(Coin {
                        denom: t.denom,
                        amount: Uint128::from_str(&t.amount)?,
                    }),
                    None => return Err(ContractError::OsmosisPoolError("failed to get pool token".to_string()))
                }
            }

            // get the total gamm shares coin
            let gamm_shares_coin = match osmo_pool.total_shares.clone() {
                Some(coin) => Coin {
                    denom: coin.denom,
                    amount: Uint128::from_str(&coin.amount)?,
                },
                None => return Err(ContractError::OsmosisPoolError(
                    "expected Some(total_shares), found None".to_string(),
                )),
            };


            // validate the price against our expectations
            // todo: remove hardcoded values and pass them as optional arguments to execute_msg
            let pool_assets_ratio = Decimal::from_ratio(pool_assets[0].amount, pool_assets[1].amount);
            if Decimal::zero() > pool_assets_ratio || Decimal::one() < pool_assets_ratio {
                return Err(ContractError::PriceRangeError {})
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

            match (asset_1_received.amount.is_zero(), asset_2_received.amount.is_zero()) {
                // both assets provided, attempt to provide two sided liquidity
                (false, false) => provide_double_sided_liquidity(
                    osmo_pool,
                    asset_1_received,
                    asset_2_received,
                    pool_assets,
                    info.sender.to_string(),
                    env.contract.address.to_string(),
                    gamm_shares_coin,
                ),
                // only asset 1 is provided, attempt to provide single sided
                (false, true) => provide_single_sided_liquidity(osmo_pool, asset_1_received),
                // only asset 2 is provided, attempt to provide single sided
                (true, false) => provide_single_sided_liquidity(osmo_pool, asset_2_received),
                // no funds provided, error out
                (true, true) => return Err(
                    ContractError::LiquidityProvisionError("no funds provided".to_string())
                ),
            }
        }
    }
}

/// cw-utils must pay requires specifically one coin, this is a helper
/// for multi-coin inputs
fn get_paid_denom_amount(info: &MessageInfo, target_denom: &str) -> StdResult<Uint128> {
    for coin in &info.funds {
        if coin.denom == target_denom {
            return Ok(coin.amount)
        }
    }
    Err(StdError::not_found(target_denom))
}


fn provide_double_sided_liquidity(
    pool: Pool,
    asset_1_paid: Coin,
    asset_2_paid: Coin,
    pool_assets: Vec<Coin>,
    sender: String,
    outpost: String,
    gamm_coin: Coin,
) -> Result<Response, ContractError> {

    let expected_gamm_shares = std::cmp::min(
        asset_1_paid.amount.multiply_ratio(
            gamm_coin.amount,
            pool_assets[0].amount,
        ),
        asset_2_paid.amount.multiply_ratio(
            gamm_coin.amount,
            pool_assets[1].amount,
        ),
    );
    let token_in_maxs: Vec<ProtoCoin> = vec![asset_1_paid.clone().into(), asset_2_paid.clone().into()];

    let osmo_msg: CosmosMsg = MsgJoinPool {
        sender: outpost,
        pool_id: pool.id,
        // exact number of shares we wish to receive
        share_out_amount: expected_gamm_shares.to_string(),
        token_in_maxs,
    }
    .into();

    let expected_gamm_shares_minus_slippage = match expected_gamm_shares.checked_multiply_ratio(
        Uint128::new(100 - 3),
        Uint128::new(100),
    ) {
        Ok(val) => val,
        Err(e) => return Err(StdError::generic_err(e.to_string()).into()),
    };

    let expected_gamm_coin = Coin {
        denom: gamm_coin.denom,
        amount: expected_gamm_shares_minus_slippage,
    };
    let gamm_transfer: CosmosMsg  = BankMsg::Send{
        to_address: sender,
        amount: vec![expected_gamm_coin],
    }
    .into();

    Ok(Response::default()
        .add_messages(vec![osmo_msg, gamm_transfer])
        .add_attribute("method", "provide_double_sided_liquidity")
        .add_attribute("pool", to_json_binary(&pool)?.to_string())
        .add_attribute("asset_1_paid", to_json_binary(&asset_1_paid)?.to_string())
        .add_attribute("asset_2_paid", to_json_binary(&asset_2_paid)?.to_string())
    )
}

fn provide_single_sided_liquidity(pool: Pool, asset_paid: Coin) -> Result<Response, ContractError> {
    Ok(Response::default()
        .add_attribute("method", "provide_single_sided_liquidity")
        .add_attribute("pool", to_json_binary(&pool)?.to_string())
        .add_attribute("asset_paid", to_json_binary(&asset_paid)?.to_string())
    )
}

fn decode_osmo_pool_binary(pool: Option<Any>) -> StdResult<Pool> {
    let osmo_shim = match pool {
        Some(shim) => shim,
        None => {
            return Err(StdError::NotFound { kind: "shim not found".to_string() })
        }
    };

    let pool: Pool = match osmo_shim.try_into() {
        Ok(result) => result,
        Err(err) => return Err(StdError::InvalidBase64 { msg: "failed to decode shim to pool".to_string() })
    };

    Ok(pool)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Err(cosmwasm_std::StdError::NotFound { kind: "not implemented".to_string() })
}
