use cosmwasm_std::{coin, Addr, Event, Storage};
use cw_multi_test::Executor;
use cw_utils::Expiration;

use crate::setup::{base_suite::BaseSuite, ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

use super::suite::SinglePartyHolderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_withdrawer() {
    SinglePartyHolderBuilder::default()
        .with_withdrawer(Some("0Oo0Oo".to_string()))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_invalid_liquid_pooler_addr() {
    SinglePartyHolderBuilder::default()
        .with_pooler_address("0Oo0Oo")
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_invalid_withdraw_to_addr() {
    SinglePartyHolderBuilder::default()
        .with_withdraw_to(Some("0Oo0Oo".to_string()))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_invalid_emergency_committee_addr() {
    SinglePartyHolderBuilder::default()
        .with_emergency_committee_addr(Some("0Oo0Oo".to_string()))
        .build();
}

#[test]
#[should_panic(expected = "The lockup period must be in the future")]
fn test_instantiate_validates_lockup_period() {
    SinglePartyHolderBuilder::default()
        .with_lockup_period(Expiration::AtHeight(1))
        .build();
}

#[test]
#[should_panic(expected = "A withdraw process already started")]
fn test_execute_claim_validates_pending_withdrawals() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    suite.expire_lockup();

    // manually setting the storage key to true
    let withdraw_state_key = "\0\u{4}wasm\0Ocontract_data/cosmos1lxsjav25s55mnxkfzkmvhdkqpsnmlm9whwk8ctqawgj438kda96s54a6mlwithdraw_state".as_bytes();
    suite.app.storage_mut().set(withdraw_state_key, "true".as_bytes());

    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "The position is still locked, unlock at: expiration: never")]
fn test_execute_claim_validates_lockup_period() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::Never {})
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "No withdrawer address configured")]
fn test_execute_claim_validates_withdrawer_set() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_withdrawer(None)
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.expire_lockup();
    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_execute_claim_validates_withdrawer_addr() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    let sender = suite.faucet.clone();
    suite.expire_lockup();
    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "No withdraw_to address configured")]
fn test_execute_claim_fails_with_no_withdraw_to() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_withdraw_to(None)
        .build();
    suite.expire_lockup();

    let sender = suite.liquid_pooler_address.clone();
    suite.expire_lockup();
    suite.execute_claim(sender);
}

#[test]
fn test_execute_claim_happy() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 0);
    suite.expire_lockup();

    suite.execute_claim(sender);

    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 2);
}

#[test]
#[should_panic(expected = "A withdraw process already started")]
fn test_execute_emergency_withdraw_validates_pending_withdrawals() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    suite.expire_lockup();

    // manually setting the storage key to true
    let withdraw_state_key = "\0\u{4}wasm\0Ocontract_data/cosmos1lxsjav25s55mnxkfzkmvhdkqpsnmlm9whwk8ctqawgj438kda96s54a6mlwithdraw_state".as_bytes();
    suite.app.storage_mut().set(withdraw_state_key, "true".as_bytes());

    suite.execute_emergency_withdraw(sender);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_execute_emergency_withdraw_validates_emergency_committee() {
    let mut suite = SinglePartyHolderBuilder::default().build();
    let sender = suite.clock.clone();
    suite.execute_emergency_withdraw(sender);
}

#[test]
fn test_execute_emergency_withdraw_happy() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 0);

    suite.expire_lockup();
    suite.execute_emergency_withdraw(sender);

    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 2);
}

#[test]
fn test_execute_distribute_validates_liquidity_pooler() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    let sender = suite.liquid_pooler_address.clone();
    let funds = vec![
        coin(1_000_000, DENOM_ATOM_ON_NTRN),
        coin(1_000_000, DENOM_LS_ATOM_ON_NTRN),
    ];
    suite.fund_contract_coins(funds.clone(), sender.clone());
    suite.expire_lockup();
    let resp = suite.execute_distribute(sender, funds);
    println!("resp: {:?}", resp);
}

#[test]
#[should_panic(expected = "No withdraw_to address configured")]
fn test_execute_distribute_validates_withdraw_to_addr() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_withdraw_to(None)
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.expire_lockup();
    suite.execute_distribute(sender, vec![]);
}

#[test]
#[should_panic(expected = "We expect 2 denoms to be received from the liquidity pooler")]
fn test_execute_distribute_ensures_two_denoms_sent() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    let sender = suite.liquid_pooler_address.clone();
    let funds = vec![coin(1_000_000, DENOM_ATOM_ON_NTRN)];
    suite.fund_contract_coins(funds.clone(), sender.clone());
    suite.expire_lockup();

    suite.execute_distribute(sender, funds);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_execute_withdraw_failed_authorizes_liquidity_pooler() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.execute_withdraw_failed(suite.clock.clone());
}

#[test]
fn test_execute_withdraw_failed_removes_withdraw_state() {
    let _suite = SinglePartyHolderBuilder::default().build();
    // todo
}

#[test]
#[should_panic(expected = "The lockup period must be in the future")]
fn test_migrate_update_config_validates_lockup_config() {
    let mut suite = SinglePartyHolderBuilder::default().build();
    let current_block = suite.get_app().block_info().height;
    let past_expiration = Expiration::AtHeight(current_block - 1);

    suite.app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.holder_addr.clone(),
            &covenant_single_party_pol_holder::msg::MigrateMsg::UpdateConfig {
                withdrawer: None,
                withdraw_to: None,
                emergency_committee: None,
                pooler_address: None,
                lockup_period: Some(past_expiration),
            },
            5,
        )
        .unwrap();
}

#[test]
fn test_migrate_update_config() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    let clock = suite.clock.to_string();

    let resp = suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.holder_addr.clone(),
            &covenant_single_party_pol_holder::msg::MigrateMsg::UpdateConfig {
                withdrawer: Some(clock.to_string()),
                withdraw_to: Some(clock.to_string()),
                emergency_committee: Some(clock.to_string()),
                pooler_address: Some(clock.to_string()),
                lockup_period: Some(Expiration::AtHeight(192837465)),
            },
            5,
        )
        .unwrap();

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute("withdrawer", clock.to_string())
            .add_attribute("withdraw_to", clock.to_string())
            .add_attribute("emergency_committee", clock.to_string())
            .add_attribute("pool_address", clock.to_string()),
    );

    let withdrawer = suite.query_withdrawer().unwrap().to_string();
    let withdraw_to = suite.query_withdraw_to().unwrap().to_string();
    let emergency_committee = suite.query_emergency_committee().unwrap().to_string();
    let pooler_address = suite.query_pooler_address().to_string();
    let lockup_period = suite.query_lockup_period();

    assert_eq!(clock, withdrawer);
    assert_eq!(clock, withdraw_to);
    assert_eq!(clock, emergency_committee);
    assert_eq!(clock, pooler_address);
    assert_eq!(Expiration::AtHeight(192837465), lockup_period);
}
