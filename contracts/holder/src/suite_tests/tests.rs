use super::is_error;
use super::suite::{SuiteBuilder, DEFAULT_WITHDRAWER};
use cosmwasm_std::{coin, coins, Addr, Coin};

#[test]
fn test_instantiate_and_query_withdrawer() {
    let suite = SuiteBuilder::default().build();
    assert_eq!(
        suite.query_withdrawer(),
        Addr::unchecked(DEFAULT_WITHDRAWER.to_string())
    );
}

#[test]
#[should_panic(expected = "Initial withdrawer is required")]
fn test_instantiate_with_no_withdrawer() {
    SuiteBuilder::default().with_withdrawer(DEFAULT_WITHDRAWER.to_string()).build();
}

#[test]
fn test_fund_contract_single_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite
        .fund_holder(user, amt_to_fund_contract.clone())
        .unwrap();

    // check that the holder contract balance has increased
    suite.assert_holder_balance(amt_to_fund_contract);
}

#[test]
fn test_fund_and_withdraw_all_unauthorized() {
    // create an attacker with an unauthorized address
    let unauthorized = Addr::unchecked("attacker");

    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // attacker attempts to withdraw all
    let resp = suite.withdraw_all(unauthorized.as_ref());
    is_error!(resp, "Unauthorized");

    // check to see the balance is unchanged
    suite.assert_holder_balance(coins(100, "coin"));
}

#[test]
fn test_fund_and_withdraw_all_single_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // withdraw all
    suite.withdraw_all(DEFAULT_WITHDRAWER).unwrap();

    // check to see there is no balance
    suite.assert_holder_balance(coins(0, "coin"));

    // and withdrawer has them all
    suite.assert_withdrawer_balance(coins(100, "coin"));
}

#[test]
fn test_fund_and_withdraw_all_two_denoms() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance: Vec<Coin> = vec![coin(100, "atom"), coin(90, "statom")];

    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract: Vec<Coin> = vec![coin(80, "atom"), coin(70, "statom")];

    suite
        .fund_holder(user, amt_to_fund_contract.clone())
        .unwrap();

    // withdraw all
    suite.withdraw_all(DEFAULT_WITHDRAWER).unwrap();

    // check to see there is no balance
    let expected_balance: Vec<Coin> = vec![coin(0, "atom"), coin(0, "statom")];

    suite.assert_holder_balance(expected_balance);

    // check to see holder has received everythning
    suite.assert_withdrawer_balance(amt_to_fund_contract);
}

#[test]
fn test_fund_and_withdraw_partial_single_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // withdraw 75 out of a total of 100 tokens
    suite
        .withdraw_tokens(DEFAULT_WITHDRAWER, coins(75, "coin"))
        .unwrap();

    // check to see there are 25 tokens left in contract
    suite.assert_holder_balance(coins(25, "coin"));

    // and holder has received 75
    suite.assert_withdrawer_balance(coins(75, "coin"));
}
#[test]
fn test_fund_and_withdraw_partial_two_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance: Vec<Coin> = vec![coin(100, "atom"), coin(90, "statom")];

    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract: Vec<Coin> = vec![coin(80, "atom"), coin(70, "statom")];

    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // withdraw partial
    let amt_to_withdraw: Vec<Coin> = vec![coin(50, "atom"), coin(30, "statom")];

    suite
        .withdraw_tokens(DEFAULT_WITHDRAWER, amt_to_withdraw.clone())
        .unwrap();

    // check to see there is subtracted balance
    let expected_balance: Vec<Coin> = vec![coin(30, "atom"), coin(40, "statom")];

    suite.assert_holder_balance(expected_balance);

    // and that withdrawer has received withdrawn amount
    suite.assert_withdrawer_balance(amt_to_withdraw);
}

#[test]
fn test_fund_and_withdraw_exact_single_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // withdraw 100 out of a total of 100 tokens
    suite
        .withdraw_tokens(DEFAULT_WITHDRAWER, coins(100, "coin"))
        .unwrap();

    // check to see there are 0 tokens left
    suite.assert_holder_balance(coins(0, "coin"));

    // and withdrawer has them all
    suite.assert_withdrawer_balance(coins(100, "coin"));
}

#[test]
fn test_fund_and_withdraw_too_big_single_denom() {
    // set up an initial user with a balance in the test suite
    let user = Addr::unchecked("anyuser");
    let initial_user_balance = coins(1000, "coin");
    let mut suite = SuiteBuilder::default()
        .with_funded_user(user.clone(), initial_user_balance)
        .build();

    // this user funds the holder contract
    let amt_to_fund_contract = coins(100, "coin");
    suite.fund_holder(user, amt_to_fund_contract).unwrap();

    // try to withdraw 200 out of a total of 100 tokens
    let resp = suite.withdraw_tokens(DEFAULT_WITHDRAWER, coins(200, "coin"));
    // the dispatched bank send message should fail and everything should roll back
    is_error!(resp, "error executing WasmMsg");

    // check to see all tokens are intact
    suite.assert_holder_balance(coins(100, "coin"));

    // and withdrawer has not received anything
    suite.assert_withdrawer_balance(coins(0, "coin"));
}
