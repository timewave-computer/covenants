use cosmwasm_std::{Addr, Coin, Uint128};

use crate::suite_test::suite::{TOKEN_A_DENOM, TOKEN_B_DENOM};

use super::suite::SuiteBuilder;

#[test]
fn test_double_sided_lp() {
    let mut suite = SuiteBuilder::default().build();

    // fund pool with balanced amounts of underlying tokens
    let token_a_amt = Uint128::new(1000);
    let token_b_amt = Uint128::new(10000);
    suite.provide_manual_liquidity("alice".to_string(), token_a_amt, token_b_amt);

    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_A_DENOM.to_string(),
        token_a_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_B_DENOM.to_string(),
        token_b_amt,
    );

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(liquid_pooler_balances[0].amount, token_a_amt);
    assert_eq!(liquid_pooler_balances[1].amount, token_b_amt);

    let liquidity_token_addr = suite
        .query_liquidity_token_addr()
        .liquidity_token
        .to_string();

    let holder_balances =
        suite.query_cw20_bal(liquidity_token_addr.clone(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);

    suite.pass_blocks(10);
    suite.tick();

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(0usize, liquid_pooler_balances.len());

    let liquid_pooler_balances =
        suite.query_cw20_bal(liquidity_token_addr, suite.liquid_pooler.1.to_string());
    assert_ne!(Uint128::zero(), liquid_pooler_balances.balance);
}

#[test]
fn test_double_and_single_sided_lp() {
    let mut suite = SuiteBuilder::default().build();

    // fund pool with balanced amounts of underlying tokens at 1:10 ratio

    suite.provide_manual_liquidity(
        "alice".to_string(),
        Uint128::new(10000),
        Uint128::new(100000),
    );

    // fund LP contract with some tokens to provide liquidity with
    let token_a_amt = Uint128::new(1000);
    let token_b_amt = Uint128::new(9000);
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_A_DENOM.to_string(),
        token_a_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_B_DENOM.to_string(),
        token_b_amt,
    );
    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(liquid_pooler_balances[0].amount, token_a_amt);
    assert_eq!(liquid_pooler_balances[1].amount, token_b_amt);

    let liquidity_token_addr = suite
        .query_liquidity_token_addr()
        .liquidity_token
        .to_string();

    let holder_balances =
        suite.query_cw20_bal(liquidity_token_addr.clone(), suite.holder_addr.to_string());
    assert_eq!(Uint128::zero(), holder_balances.balance);

    suite.pass_blocks(10);
    suite.tick();

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));

    // assert there are 100 uatoms remaining because of missmatched pool/provision ratio
    assert_eq!(
        Coin {
            denom: TOKEN_A_DENOM.to_string(),
            amount: Uint128::new(100)
        },
        liquid_pooler_balances[0],
    );
    let liquid_pooler_balance = suite
        .query_cw20_bal(
            liquidity_token_addr.to_string(),
            suite.liquid_pooler.1.to_string(),
        )
        .balance;
    assert_ne!(Uint128::zero(), liquid_pooler_balance);

    // tick again
    suite.pass_blocks(10);
    suite.tick();

    let liquid_pooler_balances =
        suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    assert_eq!(0, liquid_pooler_balances.len());

    let new_holder_balance = suite
        .query_cw20_bal(liquidity_token_addr, suite.holder_addr.to_string())
        .balance;
    assert_ne!(liquid_pooler_balance, new_holder_balance);
}

#[test]
#[should_panic(expected = "Pair type mismatch")]
fn test_migrated_pool_type_lp() {
    let mut suite = SuiteBuilder::default()
        .with_expected_pair_type(astroport::factory::PairType::Xyk {})
        .build();

    // fund pool with balanced amounts of underlying tokens
    let token_a_amt = Uint128::new(1000);
    let token_b_amt = Uint128::new(10000);
    suite.provide_manual_liquidity("alice".to_string(), token_a_amt, token_b_amt);

    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_A_DENOM.to_string(),
        token_a_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_B_DENOM.to_string(),
        token_b_amt,
    );

    suite.tick();
}

#[test]
#[should_panic(expected = "Price range error")]
fn test_lp_not_within_price_range_denom_a_dominant() {
    let mut suite = SuiteBuilder::default().build();

    // fund pool with 10:1 ratio of token a to b.
    // Liquid Pooler is configured to expect 1:10 ratio.
    // pool balances: [10000, 1000]
    suite.provide_manual_liquidity("alice".to_string(), Uint128::new(10000), Uint128::new(1000));

    let token_a_amt = Uint128::new(1000);
    let token_b_amt = Uint128::new(10000);
    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_A_DENOM.to_string(),
        token_a_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_B_DENOM.to_string(),
        token_b_amt,
    );
    suite.tick();
}

#[test]
#[should_panic(expected = "Price range error")]
fn test_lp_not_within_price_range_denom_b_dominant() {
    let mut suite = SuiteBuilder::default().build();

    // fund pool with 1:20 ratio of token a to b.
    // Liquid Pooler is configured to expect 1:10 ratio.
    // pool balances: [1000, 20000]
    suite.provide_manual_liquidity("alice".to_string(), Uint128::new(1000), Uint128::new(20000));

    let token_a_amt = Uint128::new(1000);
    let token_b_amt = Uint128::new(10000);
    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_A_DENOM.to_string(),
        token_a_amt,
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(),
        TOKEN_B_DENOM.to_string(),
        token_b_amt,
    );
    suite.tick();
}
