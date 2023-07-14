use astroport::DecimalCheckedOps;
use cosmwasm_std::{Addr, Decimal, Uint128};

use super::suite::{SuiteBuilder, NATIVE_ATOM_DENOM, ST_ATOM_DENOM};

#[test]
fn test_instantiate_happy() {
    let mut suite = SuiteBuilder::default().build();

    let redemption_rate = Decimal::from_ratio(Uint128::new(22), Uint128::new(10));
    let atom_amt = Uint128::new(400000);
    let statom_amt = atom_amt * redemption_rate;
    // fund pool with balanced amounts of underlying tokens
    suite.provide_manual_liquidity("alice".to_string(), statom_amt, atom_amt);

    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        ST_ATOM_DENOM.to_string(),
        Uint128::new(100000),
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        NATIVE_ATOM_DENOM.to_string(),
        Uint128::new(100000),
    );

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(liquid_pooler_balances[0].amount, Uint128::new(100000));
    assert_eq!(liquid_pooler_balances[1].amount, Uint128::new(100000));
    let share = suite.query_pool_info();
    println!("pool share: {:?}", share);

    let pairinfo = suite.query_liquidity_token_addr();

    let liquidity_token_addr = pairinfo.liquidity_token.to_string();

    let holder_balances = suite.query_cw20_bal(liquidity_token_addr.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);

    suite.pass_blocks(10);
    suite.tick();
    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    println!(
        "\n first tick liquid pooler balances: {:?}\n",
        liquid_pooler_balances
    );
    suite.pass_blocks(10);
    suite.tick();

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    println!(
        "\n second tick liquid pooler balances: {:?}\n",
        liquid_pooler_balances
    );

    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_ne!(Uint128::zero(), holder_balances.balance);

    suite.holder_withdraw();

    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);
    let holder_native_balances = suite.query_addr_balances(Addr::unchecked(suite.holder_addr.to_string()));
    assert_eq!(2, holder_native_balances.len());
    assert_ne!(Uint128::zero(), holder_native_balances[0].amount);
    assert_ne!(Uint128::zero(), holder_native_balances[1].amount);
}

// tests todo:
// 1. randomly funded contracts/wallets
// 2. existing pool ratios (imbalanced, equal, extremely imbalanced, providing more liq than exists)

#[test]
fn test_exceeded_single_side_lp_ratio_first_asset_dominant() {
    // here we try to provide liquidity but end up with some leftover assets
    // at that point ticking should effectively achieve nothing
    // once multiple ticks happen and we are sure nothign happens,
    // we fund the contract with some counterpart asset.
    // this should enable double side liquidity to be provided,
    // and any leftovers to be LP'd via single sided liquidity
    let mut suite = SuiteBuilder::default().build();

    let redemption_rate = Decimal::from_ratio(Uint128::new(10), Uint128::new(13));
    let atom_amt = Uint128::new(100000);
    let statom_amt = atom_amt * redemption_rate;

    suite.provide_manual_liquidity("alice".to_string(), statom_amt, atom_amt);

    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        ST_ATOM_DENOM.to_string(),
        statom_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        NATIVE_ATOM_DENOM.to_string(),
        atom_amt,
    );

    let pairinfo = suite.query_liquidity_token_addr();
    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);

    suite.tick();
    suite.pass_blocks(10);

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));

    println!("lp balances: {:?}", liquid_pooler_balances);
    suite.tick();
    suite.pass_blocks(10);

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));

    println!("lp balances: {:?}", liquid_pooler_balances);

    suite.tick();
    suite.tick();
    suite.tick();
    suite.pass_blocks(10);

    assert_eq!(
        liquid_pooler_balances,
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()))
    );

    // given our single-side lp limit is 100 tokens and there are 148stuatom remaining,
    // we fund the contract with 100 uatom. this should enable double sided liquidity to be
    // provided, and result in a leftoveramount <= 100 to single-side lp
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        NATIVE_ATOM_DENOM.to_string(),
        Uint128::new(100),
    );
    suite.tick();
    suite.tick();
    suite.pass_blocks(10);

    suite.tick();
    suite.tick();
    suite.pass_blocks(10);

    assert_eq!(
        0,
        suite
            .query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()))
            .len()
    );
    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_ne!(Uint128::zero(), holder_balances.balance);

    suite.holder_withdraw();

    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);
    let holder_native_balances = suite.query_addr_balances(Addr::unchecked(suite.holder_addr.to_string()));
    assert_eq!(2, holder_native_balances.len());
    assert_ne!(Uint128::zero(), holder_native_balances[0].amount);
    assert_ne!(Uint128::zero(), holder_native_balances[1].amount);
}

#[test]
fn test_exceeded_single_side_lp_ratio_second_asset_dominant() {
    let mut suite = SuiteBuilder::default().build();

    let redemption_rate = Decimal::from_ratio(Uint128::new(103), Uint128::new(100));
    let atom_amt = Uint128::new(100000);
    let statom_amt = redemption_rate.checked_mul_uint128(atom_amt).unwrap();

    suite.provide_manual_liquidity("alice".to_string(), statom_amt, atom_amt);
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        ST_ATOM_DENOM.to_string(),
        statom_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        NATIVE_ATOM_DENOM.to_string(),
        atom_amt,
    );

    let pairinfo = suite.query_liquidity_token_addr();
    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);

    suite.tick();
    suite.tick();
    suite.tick();
    suite.pass_blocks(10);

    let balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(1, balances.len());
    let intervention_amount = balances[0].amount + Uint128::new(40);

    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        NATIVE_ATOM_DENOM.to_string(),
        intervention_amount,
    );
    suite.tick();
    suite.tick();
    suite.tick();
    suite.tick();
    suite.pass_blocks(10);

    // if there are no more balances, everything is LP
    assert_eq!(
        0,
        suite
            .query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()))
            .len()
    );
    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_ne!(Uint128::zero(), holder_balances.balance);

    suite.holder_withdraw();

    let holder_balances = suite.query_cw20_bal(pairinfo.liquidity_token.to_string(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);
    let holder_native_balances = suite.query_addr_balances(Addr::unchecked(suite.holder_addr.to_string()));
    assert_eq!(2, holder_native_balances.len());
    assert_ne!(Uint128::zero(), holder_native_balances[0].amount);
    assert_ne!(Uint128::zero(), holder_native_balances[1].amount);
}
