use astroport::asset::PairInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{QuerierWrapper, StdError, Uint128};
use cw20::BalanceResponse;

/// queries the liquidity token balance of given address
pub fn query_liquidity_token_balance(
    querier: QuerierWrapper,
    liquidity_token: &str,
    contract_addr: String,
) -> Result<Uint128, StdError> {
    let liquidity_token_balance: BalanceResponse = querier.query_wasm_smart(
        liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: contract_addr,
        },
    )?;
    Ok(liquidity_token_balance.balance)
}

/// queries the cw20 liquidity token address corresponding to a given pool
pub fn query_liquidity_token_address(
    querier: QuerierWrapper,
    pool: String,
) -> Result<String, StdError> {
    let pair_info: PairInfo =
        querier.query_wasm_smart(pool, &astroport::pair::QueryMsg::Pair {})?;
    Ok(pair_info.liquidity_token.to_string())
}

pub fn query_astro_pool_token(
    querier: QuerierWrapper,
    pool: String,
    addr: String,
) -> Result<AstroportPoolTokenResponse, StdError> {
    let pair_info: PairInfo =
        querier.query_wasm_smart(pool, &astroport::pair::QueryMsg::Pair {})?;

    let liquidity_token_balance: BalanceResponse = querier.query_wasm_smart(
        pair_info.liquidity_token.as_ref(),
        &cw20::Cw20QueryMsg::Balance { address: addr },
    )?;

    Ok(AstroportPoolTokenResponse {
        pair_info,
        balance_response: liquidity_token_balance,
    })
}

#[cw_serde]
pub struct AstroportPoolTokenResponse {
    pub pair_info: PairInfo,
    pub balance_response: BalanceResponse,
}
