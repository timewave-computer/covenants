use std::str::FromStr;

use cosmwasm_std::{coin, coins, Decimal, Uint128, Uint64};
use valence_outpost_osmo_liquid_pooler::msg::{
    OutpostProvideLiquidityConfig, OutpostWithdrawLiquidityConfig,
};

use crate::{
    setup::{base_suite::BaseSuiteMut, DENOM_ATOM, DENOM_FALLBACK, DENOM_LS_ATOM_ON_NTRN},
    test_osmo_lp_outpost::suite::OsmoLpOutpostBuilder,
};

// TODO: these tests are incomplete and should be expanded
#[test]
fn test_withdraw_liquidity() {
    let mut suite = OsmoLpOutpostBuilder::default().build();

    suite.fund_contract(&coins(1, DENOM_ATOM), suite.outpost.clone());
    suite.fund_contract(&coins(1, DENOM_LS_ATOM_ON_NTRN), suite.outpost.clone());

    suite.withdraw_liquidity(
        coins(1, DENOM_FALLBACK),
        suite.faucet.clone(),
        OutpostWithdrawLiquidityConfig {
            pool_id: Uint64::new(1),
        },
    );
}

#[test]
fn test_provide_liquidity_double_sided() {
    let mut suite = OsmoLpOutpostBuilder::default().build();

    suite.fund_contract(&coins(1, DENOM_ATOM), suite.outpost.clone());
    suite.fund_contract(&coins(1, DENOM_LS_ATOM_ON_NTRN), suite.outpost.clone());

    suite.provide_liquidity(
        vec![coin(1, DENOM_ATOM), coin(1, DENOM_LS_ATOM_ON_NTRN)],
        suite.faucet.clone(),
        OutpostProvideLiquidityConfig {
            pool_id: Uint64::new(1),
            expected_spot_price: Decimal::from_str("1.0").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.01").unwrap(),
            slippage_tolerance: Decimal::from_str("0.01").unwrap(),
            asset_1_single_side_lp_limit: Uint128::new(100000),
            asset_2_single_side_lp_limit: Uint128::new(100000),
        },
    );
}

#[test]
fn test_provide_liquidity_single_sided_asset_a() {
    let mut suite = OsmoLpOutpostBuilder::default().build();

    suite.fund_contract(&coins(1, DENOM_ATOM), suite.outpost.clone());
    suite.fund_contract(&coins(1, DENOM_LS_ATOM_ON_NTRN), suite.outpost.clone());

    suite.provide_liquidity(
        coins(1, DENOM_ATOM),
        suite.faucet.clone(),
        OutpostProvideLiquidityConfig {
            pool_id: Uint64::new(1),
            expected_spot_price: Decimal::from_str("1.0").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.01").unwrap(),
            slippage_tolerance: Decimal::from_str("0.01").unwrap(),
            asset_1_single_side_lp_limit: Uint128::new(100000),
            asset_2_single_side_lp_limit: Uint128::new(100000),
        },
    );
}

#[test]
fn test_provide_liquidity_single_sided_asset_b() {
    let mut suite = OsmoLpOutpostBuilder::default().build();

    suite.fund_contract(&coins(1, DENOM_ATOM), suite.outpost.clone());
    suite.fund_contract(&coins(1, DENOM_LS_ATOM_ON_NTRN), suite.outpost.clone());

    suite.provide_liquidity(
        coins(1, DENOM_LS_ATOM_ON_NTRN),
        suite.faucet.clone(),
        OutpostProvideLiquidityConfig {
            pool_id: Uint64::new(1),
            expected_spot_price: Decimal::from_str("1.0").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.01").unwrap(),
            slippage_tolerance: Decimal::from_str("0.01").unwrap(),
            asset_1_single_side_lp_limit: Uint128::new(100000),
            asset_2_single_side_lp_limit: Uint128::new(100000),
        },
    );
}
