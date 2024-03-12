use cosmwasm_std::{coin, Addr, Event};
use cw_multi_test::Executor;

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
// #[should_panic(expected = "A withdraw process already started")]
fn test_execute_claim_validates_pending_withdrawals() {
    // TODO: enable should_panic
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
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .with_withdrawer(None)
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_execute_claim_validates_withdrawer_addr() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    let sender = suite.faucet.clone();
    suite.execute_claim(sender);
}

#[test]
#[should_panic(expected = "No withdraw_to address configured")]
fn test_execute_claim_fails_with_no_withdraw_to() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_withdraw_to(None)
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.execute_claim(sender);
}

#[test]
fn test_execute_claim_happy() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 0);

    suite.execute_claim(sender);

    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 2);
}

#[test]
fn test_execute_emergency_withdraw_validates_pending_withdrawals() {
    let _suite = SinglePartyHolderBuilder::default().build();
    // todo: should panic
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
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    suite.enter_pool();

    let sender = suite.liquid_pooler_address.clone();
    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 0);

    suite.execute_emergency_withdraw(sender);

    let bals = suite.query_all_balances(&suite.liquid_pooler_address);
    assert_eq!(bals.len(), 2);
}

#[test]
fn test_execute_distribute_validates_liquidity_pooler() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    let sender = suite.liquid_pooler_address.clone();
    let funds = vec![
        coin(1_000_000, DENOM_ATOM_ON_NTRN),
        coin(1_000_000, DENOM_LS_ATOM_ON_NTRN),
    ];
    suite.fund_contract_coins(funds.clone(), sender.clone());

    let resp = suite.execute_distribute(sender, funds);
    println!("resp: {:?}", resp);
}

#[test]
#[should_panic(expected = "No withdraw_to address configured")]
fn test_execute_distribute_validates_withdraw_to_addr() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .with_withdraw_to(None)
        .build();

    let sender = suite.liquid_pooler_address.clone();
    suite.execute_distribute(sender, vec![]);
}

#[test]
#[should_panic(expected = "We expect 2 denoms to be received from the liquidity pooler")]
fn test_execute_distribute_ensures_two_denoms_sent() {
    let mut suite = SinglePartyHolderBuilder::default()
        .with_lockup_period(cw_utils::Expiration::AtHeight(12312))
        .build();

    let sender = suite.liquid_pooler_address.clone();
    let funds = vec![coin(1_000_000, DENOM_ATOM_ON_NTRN)];
    suite.fund_contract_coins(funds.clone(), sender.clone());

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
                lockup_period: None,
            },
            5,
        )
        .unwrap();

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute("withdrawer", clock.to_string())
            .add_attribute("withdraw_to", clock.to_string())
            .add_attribute("emergency_committee", clock.to_string())
            .add_attribute("pool_address", clock),
    );
}
