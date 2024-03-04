use cosmwasm_std::Uint128;
use covenant_utils::SingleSideLpLimits;

use super::suite::{AstroLiquidPoolerBuilder, Suite};

#[test]
fn test_instantiate_validates_addresses() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_instantiate_validates_pool_price_config() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_instantiate_happy() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_withdraw_percentage_range() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_withdraw_validates_holder() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_withdraw_no_lp_tokens() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_withdraw_happy() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_validates_pair_type() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_validates_pool_assets() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_validates_pool_ratio() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_no_assets() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_single_side_asset_a() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_single_side_asset_a_exceeds_limits() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_single_side_asset_b_happy() {
    let mut builder = AstroLiquidPoolerBuilder::default();

    builder
        .with_single_side_lp_limits(SingleSideLpLimits {
        asset_a_limit: Uint128::new(43210),
        asset_b_limit: Uint128::new(50000),
    });

    let suite = builder.finalize();


    println!("suite final config: {:?}", suite.liquid_pooler_addr);
            // .with_single_side_lp_limits(SingleSideLpLimits {
            //     asset_a_limit: Uint128::new(43210),
            //     asset_b_limit: Uint128::new(50000),
            // })
            // .finalize();
    // println!("lp_config: {:?}", suite.lp_config);
    // TODO
}

#[test]
fn test_provide_liquidity_single_side_asset_b_exceeds_limits() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_provide_liquidity_double_side() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}

#[test]
fn test_migrate_update_config() {
    let mut suite = AstroLiquidPoolerBuilder::default().finalize();
    // TODO
}
