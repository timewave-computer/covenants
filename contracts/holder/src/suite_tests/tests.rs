use super::suite::{SuiteBuilder, DEFAULT_WITHDRAWER};
use cosmwasm_std::{coin, coins, Addr};

#[test]
fn test_instantiate_and_query_withdrawer() {
    let suite = SuiteBuilder::default().build();
    assert_eq!(
        suite.query_withdrawer(),
        Addr::unchecked(DEFAULT_WITHDRAWER.to_string())
    );
}

#[test]
#[should_panic(expected = "Invalid input: address not normalized")]
fn test_instantiate_invalid_withdrawer() {
    SuiteBuilder::default()
        .with_withdrawer("0Oo0Oo".to_string())
        .build();
}

#[test]
#[should_panic(expected = "Invalid input: address not normalized")]
fn test_instantiate_invalid_lp_addr() {
    SuiteBuilder::default()
        .with_lp("0Oo0Oo".to_string())
        .build();
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_withdraw_all_unauthorized() {
    let mut suite = SuiteBuilder::default().build();

    suite.fund_holder(coins(100, "coin"));

    // attacker attempts to withdraw, panic
    suite.withdraw_all("attacker").unwrap();
}

#[test]
fn test_withdraw_all_single_denom() {
    let mut suite = SuiteBuilder::default().build();

    suite.fund_holder(coins(100, "coin"));

    // withdraw all
    suite.withdraw_all(DEFAULT_WITHDRAWER).unwrap();

    // check to see there is no balance
    suite.assert_holder_balance(coins(0, "coin"));

    // and withdrawer has them all
    suite.assert_withdrawer_balance(coins(100, "coin"));
}

#[test]
fn test_withdraw_all_two_denoms() {
    let mut suite = SuiteBuilder::default().build();

    let balances = vec![coin(80, "atom"), coin(70, "statom")];
    suite.fund_holder(balances.clone());

    // withdraw all
    suite.withdraw_all(DEFAULT_WITHDRAWER).unwrap();

    // assert all funds are now in withdrawer address
    suite.assert_holder_balance(vec![coin(0, "atom"), coin(0, "statom")]);
    suite.assert_withdrawer_balance(balances);
}

#[test]
fn test_fund_single_withdraw_partial_single_denom() {
    let mut suite = SuiteBuilder::default().build();

    suite.fund_holder(vec![coin(80, "atom")]);

    // withdraw 75 out of a total of 100 tokens
    suite.withdraw_tokens(DEFAULT_WITHDRAWER, coins(75, "atom"));

    // check to see there are 25 tokens left in contract
    suite.assert_holder_balance(coins(5, "atom"));

    // and holder has received 75
    suite.assert_withdrawer_balance(coins(75, "atom"));
}
#[test]
fn test_fund_multi_denom_withdraw_partial_two_denom() {
    let mut suite = SuiteBuilder::default().build();

    let balances = vec![coin(80, "atom"), coin(70, "statom")];
    suite.fund_holder(balances);

    let amt_to_withdraw = vec![coin(50, "atom"), coin(30, "statom")];

    suite.withdraw_tokens(DEFAULT_WITHDRAWER, amt_to_withdraw.clone());

    let expected_balance = vec![coin(30, "atom"), coin(40, "statom")];
    suite.assert_holder_balance(expected_balance);
    suite.assert_withdrawer_balance(amt_to_withdraw);
}

#[test]
fn test_fund_multi_denom_withdraw_exact_single_denom() {
    let mut suite = SuiteBuilder::default().build();

    let balances = vec![coin(80, "atom"), coin(70, "stuatom")];
    suite.fund_holder(balances);

    suite.withdraw_tokens(DEFAULT_WITHDRAWER, coins(70, "stuatom"));

    // check to see there are 0 tokens left
    suite.assert_holder_balance(vec![coin(80, "atom")]);

    suite.assert_withdrawer_balance(coins(70, "stuatom"));
}

#[test]
#[should_panic(expected = "Cannot Sub with 70 and 100")]
fn test_fund_single_and_withdraw_too_big_single_denom() {
    let mut suite = SuiteBuilder::default().build();
    let holder_balances = vec![coin(80, "atom"), coin(70, "statom")];
    suite.fund_holder(holder_balances);

    suite.withdraw_tokens(DEFAULT_WITHDRAWER, coins(100, "statom"));
}
