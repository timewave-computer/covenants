use std::str::FromStr;

use cosmwasm_std::{coin, coins, Addr, Decimal, Event, Uint128};
use covenant_utils::PoolPriceConfig;
use cw_multi_test::Executor;
use valence_astroport_liquid_pooler::msg::{AssetData, ProvidedLiquidityInfo};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
};

use super::suite::AstroLiquidPoolerBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_clock_address() {
    AstroLiquidPoolerBuilder::default()
        .with_clock_address("not a clock".to_string())
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_pool_address() {
    AstroLiquidPoolerBuilder::default()
        .with_pool_address("not a pool".to_string())
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_holder_address() {
    AstroLiquidPoolerBuilder::default()
        .with_holder_address("not a holder".to_string())
        .build();
}

#[test]
#[should_panic(expected = "Cannot Sub with 1 and 2")]
fn test_instantiate_validates_pool_price_config_upper_bound() {
    AstroLiquidPoolerBuilder::default()
        .with_pool_price_config(PoolPriceConfig {
            expected_spot_price: Decimal::from_str("1.0").unwrap(),
            acceptable_price_spread: Decimal::from_str("2.0").unwrap(),
        })
        .build();
}

#[test]
#[should_panic(expected = "Cannot Sub with 0.5 and 0.6")]
fn test_instantiate_validates_pool_price_config_lower_bound() {
    AstroLiquidPoolerBuilder::default()
        .with_pool_price_config(PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.5").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.6").unwrap(),
        })
        .build();
}

#[test]
#[should_panic(expected = "Pair type mismatch")]
fn test_instantiate_validates_pool_pair_type() {
    AstroLiquidPoolerBuilder::default()
        .with_custom_astroport_pool(
            astroport::factory::PairType::Xyk {},
            coin(1_000_000, DENOM_ATOM_ON_NTRN),
            coin(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        )
        .build();
}

#[test]
#[should_panic(expected = "Withdraw percentage range must belong to range (0.0, 1.0]")]
fn test_withdraw_validates_percentage_range_ceiling() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
    suite.expire_lockup();
    let holder: Addr = suite.holder_addr.clone();
    suite
        .app
        .execute_contract(
            holder.clone(),
            suite.liquid_pooler_addr.clone(),
            &valence_astroport_liquid_pooler::msg::ExecuteMsg::Withdraw {
                percentage: Some(Decimal::from_str("101.0").unwrap()),
            },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Withdraw percentage range must belong to range (0.0, 1.0]")]
fn test_withdraw_validates_percentage_range_floor() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    let holder = suite.holder_addr.clone();
    suite
        .app
        .execute_contract(
            holder.clone(),
            suite.liquid_pooler_addr.clone(),
            &valence_astroport_liquid_pooler::msg::ExecuteMsg::Withdraw {
                percentage: Some(Decimal::from_str("0.0").unwrap()),
            },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Only holder can withdraw the position")]
fn test_withdraw_validates_holder() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    let not_the_holder = suite.faucet.clone();

    suite
        .app
        .execute_contract(
            not_the_holder,
            suite.liquid_pooler_addr.clone(),
            &valence_astroport_liquid_pooler::msg::ExecuteMsg::Withdraw { percentage: None },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "no covenant denom or lp tokens available")]
fn test_withdraw_no_lp_or_covenant_denoms() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    let withdrawer = suite.clock_addr.clone();
    suite.expire_lockup();
    suite.withdraw(&withdrawer, None);
}

#[test]
fn test_withdraw_no_lp_tokens_withdraws_covenant_assets() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    let withdrawer = suite.clock_addr.clone();

    suite.assert_balance(suite.holder_addr.clone(), coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(suite.holder_addr.clone(), coin(0, DENOM_LS_ATOM_ON_NTRN));
    suite.expire_lockup();
    suite.withdraw(&withdrawer, None);

    suite.assert_balance(suite.holder_addr.clone(), coin(500_000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(
        suite.holder_addr.clone(),
        coin(500_000, DENOM_LS_ATOM_ON_NTRN),
    );
}

#[test]
fn test_withdraw_no_percentage_defaults_to_full_position() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    let withdrawer = suite.clock_addr.clone();
    let holder = suite.holder_addr.clone();

    suite.fund_contract(
        &coins(500_001, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_001, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
    suite.expire_lockup();
    suite.withdraw(&withdrawer, None);

    suite.assert_balance(&holder, coin(500_000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&holder, coin(500_000, DENOM_LS_ATOM_ON_NTRN));
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_tick_unauthorized() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    let unauthorized_sender = suite.admin.clone();

    suite
        .app
        .execute_contract(
            unauthorized_sender,
            suite.liquid_pooler_addr.clone(),
            &valence_clock::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Pair type mismatch")]
fn test_provide_liquidity_validates_pair_type() {
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_pair_type(astroport::factory::PairType::Xyk {})
        .build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
}

#[test]
#[should_panic(expected = "all pool assets must be non-zero")]
fn test_provide_liquidity_determine_pool_ratio_asset_b_denom_invalid() {
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_assets(AssetData {
            asset_a_denom: DENOM_ATOM_ON_NTRN.to_string(),
            asset_b_denom: "invalid denom".to_string(),
        })
        .build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
}

#[test]
#[should_panic(expected = "Price range error")]
fn test_provide_liquidity_validates_pool_ratio() {
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_pool_price_config(PoolPriceConfig {
            expected_spot_price: Decimal::from_str("3").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.1").unwrap(),
        })
        .build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
}

#[test]
fn test_provide_liquidity_no_assets() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite
        .tick_contract(suite.liquid_pooler_addr.clone())
        .assert_event(&Event::new("wasm").add_attribute("status", "not enough funds"));
}

#[test]
fn test_provide_stable_liquidity_single_side_asset_a() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(570_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    suite
        .tick_contract(suite.liquid_pooler_addr.clone())
        .assert_event(&Event::new("wasm").add_attribute("method", "single_side_lp"));
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(570_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_custom_concentrated_liquidity_single_side_asset_a() {
    let custom_concentrated_pair_type =
        astroport::factory::PairType::Custom("concentrated".to_string());
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_custom_astroport_pool(
            custom_concentrated_pair_type.clone(),
            coin(1_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(1_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        )
        .with_pair_type(custom_concentrated_pair_type)
        .build();

    suite.fund_contract(
        &coins(570_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    suite
        .tick_contract(suite.liquid_pooler_addr.clone())
        .assert_event(&Event::new("wasm").add_attribute("method", "single_side_lp"));
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(570_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_xyk_liquidity_single_side_asset_a() {
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_custom_astroport_pool(
            astroport::factory::PairType::Xyk {},
            coin(1_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(1_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        )
        .with_pair_type(astroport::factory::PairType::Xyk {})
        .build();

    suite.fund_contract(
        &coins(570_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    // first tick double-side lps
    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    // second tick swaps and then double side lps
    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );

    let provided_liquidity_info = suite.query_provided_liquidity_info();
    assert_eq!(
        provided_liquidity_info.provided_coin_a.amount,
        Uint128::new(500_000 + 70_000 / 2)
    );
    // minus fees
    assert!(provided_liquidity_info.provided_coin_b.amount > Uint128::new(534_000));
}

#[test]
fn test_provide_xyk_liquidity_single_side_asset_b() {
    let mut suite = AstroLiquidPoolerBuilder::default()
        .with_custom_astroport_pool(
            astroport::factory::PairType::Xyk {},
            coin(1_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(1_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        )
        .with_pair_type(astroport::factory::PairType::Xyk {})
        .build();

    suite.fund_contract(
        &coins(570_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    // first tick double-side lps
    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_LS_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    // second tick swaps and then double side lps
    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );

    let provided_liquidity_info = suite.query_provided_liquidity_info();
    assert_eq!(
        provided_liquidity_info.provided_coin_b.amount,
        Uint128::new(500_000 + 70_000 / 2)
    );
    // minus fees
    assert!(provided_liquidity_info.provided_coin_a.amount > Uint128::new(534_000));
}

#[test]
#[should_panic(expected = "Single side LP limit exceeded")]
fn test_provide_liquidity_single_side_asset_a_exceeds_limits() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(500_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_liquidity_single_side_validates_single_side_limits() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(570_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    let double_sided_response = suite.tick_contract(suite.liquid_pooler_addr.clone());
    double_sided_response
        .assert_event(&Event::new("wasm").add_attribute("method", "double_side_lp"));
    double_sided_response
        .assert_event(&Event::new("wasm").add_attribute("method", "handle_double_sided_reply_id"));

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    let app_response = suite.tick_contract(suite.liquid_pooler_addr.clone());
    app_response.assert_event(&Event::new("wasm").add_attribute("method", "single_side_lp"));
    app_response
        .assert_event(&Event::new("wasm").add_attribute("method", "handle_single_sided_reply_id"));

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(570_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_liquidity_single_side_asset_b() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(570_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    let double_sided_response = suite.tick_contract(suite.liquid_pooler_addr.clone());
    double_sided_response
        .assert_event(&Event::new("wasm").add_attribute("method", "double_side_lp"));
    double_sided_response
        .assert_event(&Event::new("wasm").add_attribute("method", "handle_double_sided_reply_id"));

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(70_000, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    let app_response = suite.tick_contract(suite.liquid_pooler_addr.clone());
    app_response.assert_event(&Event::new("wasm").add_attribute("method", "single_side_lp"));
    app_response
        .assert_event(&Event::new("wasm").add_attribute("method", "handle_single_sided_reply_id"));

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(570_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
#[should_panic(expected = "Single side LP limit exceeded")]
fn test_provide_liquidity_single_side_asset_b_exceeds_limits() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(500_000, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_liquidity_double_side_excess_a_denom() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(1_000_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(500_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(500_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_provide_liquidity_double_side_excess_b_denom() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();

    suite.fund_contract(
        &coins(500_000, DENOM_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );
    suite.fund_contract(
        &coins(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        suite.liquid_pooler_addr.clone(),
    );

    suite.tick_contract(suite.liquid_pooler_addr.clone());

    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        suite.liquid_pooler_addr.clone(),
        coin(500_000, DENOM_LS_ATOM_ON_NTRN),
    );
    assert_eq!(
        suite.query_provided_liquidity_info(),
        ProvidedLiquidityInfo {
            provided_coin_a: coin(500_000, DENOM_ATOM_ON_NTRN),
            provided_coin_b: coin(500_000, DENOM_LS_ATOM_ON_NTRN)
        }
    );
}

#[test]
fn test_migrate_update_config() {
    let mut suite = AstroLiquidPoolerBuilder::default().build();
    let liquid_pooler = suite.liquid_pooler_addr.clone();
    let clock = suite.clock_addr.clone();
    let holder = suite.holder_addr.clone();
    let mut lp_config = suite.lp_config.clone();
    lp_config.pair_type = astroport::factory::PairType::Xyk {};

    // swap clock & holder, and update pair type
    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            liquid_pooler,
            &valence_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
                clock_addr: Some(holder.to_string()),
                holder_address: Some(clock.to_string()),
                lp_config: Some(Box::new(lp_config)),
            },
            11,
        )
        .unwrap();

    let lp_config = suite.query_lp_config();
    let holder_address = suite.query_holder_address();
    let clock_address = suite.query_clock_address();
    let contract_state = suite.query_contract_state();

    assert_eq!(lp_config.pair_type, astroport::factory::PairType::Xyk {});
    assert_eq!(holder_address, clock);
    assert_eq!(clock_address, holder);
    assert_eq!(
        contract_state,
        valence_astroport_liquid_pooler::msg::ContractState::Instantiated {}
    );
}
