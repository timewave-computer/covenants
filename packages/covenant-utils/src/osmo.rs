use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128, Uint64};

#[cw_serde]
pub struct OutpostProvideLiquidityConfig {
    /// id of the pool we wish to provide liquidity to
    pub pool_id: Uint64,
    /// the price which we expect to provide liquidity at
    pub expected_spot_price: Decimal,
    /// acceptable delta (both ways) of the expected price
    pub acceptable_price_spread: Decimal,
    /// slippage tolerance
    pub slippage_tolerance: Decimal,
    /// limits for single-side liquidity provision
    pub asset_1_single_side_lp_limit: Uint128,
    pub asset_2_single_side_lp_limit: Uint128,
}

#[cw_serde]
pub enum OutpostExecuteMsg {
    ProvideLiquidity {
        config: OutpostProvideLiquidityConfig,
    },
}
