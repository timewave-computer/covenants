use astroport::router;
use cosmwasm_std::{Timestamp, Uint128};
use covenant_utils::LockupConfig;

use crate::{suite_tests::suite::{CLOCK_ADDR, POOL, NEXT_CONTRACT, PARTY_A_ROUTER, PARTY_B_ROUTER, Suite, get_default_block_info}, msg::ContractState};

use super::suite::SuiteBuilder;

#[test]
fn test_instantiate_happy_and_query_all() {
    let suite = SuiteBuilder::default().build();
    let clock = suite.query_clock_address();
    let pool: cosmwasm_std::Addr = suite.query_pool();
    let next_contract = suite.query_next_contract();
    let party_a_router = suite.query_router_party_a();
    let party_b_router = suite.query_router_party_b();
    let deposit_deadline = suite.query_deposit_deadline();
    let contract_state = suite.query_contract_state();

    assert_eq!(ContractState::Instantiated, contract_state);
    assert_eq!(CLOCK_ADDR, clock);
    assert_eq!(POOL, pool);
    assert_eq!(NEXT_CONTRACT, next_contract.to_string());
    assert_eq!(PARTY_A_ROUTER, party_a_router.to_string());
    assert_eq!(PARTY_B_ROUTER, party_b_router.to_string());
    assert_eq!(LockupConfig::None, deposit_deadline);
}

#[test]
#[should_panic(expected = "block height must be in the future")]
fn test_instantiate_invalid_deposit_deadline_block_based() {
    SuiteBuilder::default()
        .with_deposit_deadline(LockupConfig::Block(1))
        .build();
}

#[test]
#[should_panic(expected = "block time must be in the future")]
fn test_instantiate_invalid_deposit_deadline_time_based() {
    SuiteBuilder::default()
        .with_deposit_deadline(LockupConfig::Time(Timestamp::from_nanos(1)))
        .build();
}

#[test]
fn test_instantiate_invalid_lockup_config() {
    let suite = SuiteBuilder::default().build();
   
}

#[test]
fn test_single_party_deposit_refund_block_based() {
    let mut suite = SuiteBuilder::default()
        .with_deposit_deadline(LockupConfig::Block(12545))
        .build();
    
    // party A fulfills their part of covenant but B fails to
    let coin = suite.get_party_a_coin(Uint128::new(500));
    suite.fund_coin(coin);

    // time passes, clock ticks..
    suite.pass_blocks(250);
    suite.tick(CLOCK_ADDR).unwrap();

    let holder_balance = suite.get_denom_a_balance(suite.holder.to_string());
    let router_a_balance = suite.get_denom_a_balance(
        suite.query_router_party_a().to_string());
    let holder_state = suite.query_contract_state();

    assert_eq!(ContractState::Complete, holder_state);
    assert_eq!(Uint128::zero(), holder_balance);
    assert_eq!(Uint128::new(500), router_a_balance);
}

#[test]
fn test_single_party_deposit_refund_time_based() {
    let current_timestamp = get_default_block_info();
    let mut suite = SuiteBuilder::default()
        .with_deposit_deadline(LockupConfig::Time(current_timestamp.time.plus_minutes(200)))
        .build();
    
    // party A fulfills their part of covenant but B fails to
    let coin = suite.get_party_a_coin(Uint128::new(500));
    suite.fund_coin(coin);

    // time passes, clock ticks..
    suite.pass_minutes(250);
    suite.tick(CLOCK_ADDR).unwrap();

    let holder_balance = suite.get_denom_a_balance(suite.holder.to_string());
    let router_a_balance = suite.get_denom_a_balance(
        suite.query_router_party_a().to_string());
    let holder_state = suite.query_contract_state();

    assert_eq!(ContractState::Complete, holder_state);
    assert_eq!(Uint128::zero(), holder_balance);
    assert_eq!(Uint128::new(500), router_a_balance);
}


#[test]
fn test_single_party_deposit_refund_no_deposit_deadline() {
    let mut suite = SuiteBuilder::default().build();
    
    // party A fulfills their part of covenant but B fails to
    let coin = suite.get_party_a_coin(Uint128::new(500));
    suite.fund_coin(coin);

    // time passes, clock ticks..
    suite.pass_minutes(25000000);
    suite.tick(CLOCK_ADDR).unwrap();

    // we assert that holder still holds the tokens and did not advance the state
    let holder_balance = suite.get_denom_a_balance(suite.holder.to_string());
    let holder_state = suite.query_contract_state();

    assert_eq!(ContractState::Instantiated, holder_state);
    assert_eq!(Uint128::new(500), holder_balance);
}
