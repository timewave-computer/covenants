use cosmwasm_std::{coins, Addr, Event, Uint128};
use covenant_swap_holder::msg::ContractState;
use covenant_utils::{CovenantTerms, SwapCovenantTerms};
use cw_multi_test::Executor;
use cw_utils::Expiration;

use crate::setup::{base_suite::BaseSuiteMut, ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

use super::suite::SwapHolderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_next_contract() {
    SwapHolderBuilder::default()
        .with_next_contract("invalid_address")
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_clock_address() {
    SwapHolderBuilder::default()
        .with_clock_address("invalid_address")
        .build();
}

#[test]
#[should_panic(expected = "past lockup config")]
fn test_instantiate_validates_lockup_config() {
    SwapHolderBuilder::default()
        .with_lockup_config(Expiration::AtHeight(0))
        .build();
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_execute_tick_validates_clock() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.app.execute_contract(
        suite.admin,
        suite.holder.clone(),
        &covenant_swap_holder::msg::ExecuteMsg::Tick {},
        &[],
    )
    .unwrap();
}

#[test]
fn test_execute_tick_instantiated_expires() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.expire_lockup_config();
    suite.tick_contract(suite.holder.clone());

    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired{}));
}

#[test]
#[should_panic(expected = "Insufficient funds to forward")]
fn test_execute_tick_instantiated_validates_sufficient_funds() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.tick_contract(suite.holder.clone());
}

#[test]
// #[should_panic(expected = "Next contract is not ready for receiving the funds yet")]
fn test_execute_tick_instantiated_validates_next_contract_deposit_addr() {
    // let mut suite = SwapHolderBuilder::default().build();

    // suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.holder.clone());
    // suite.fund_contract(&coins(100000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());

    // suite.tick_contract(suite.holder.clone());
}

#[test]
fn test_execute_tick_instantiated_forwards_and_completes() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.holder.clone());
    suite.fund_contract(&coins(100000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());

    suite.tick_contract(suite.holder.clone());

    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Complete{}));
}

#[test]
fn test_execute_expired_refund_both_parties() {

}


#[test]
fn test_execute_expired_refund_party_a() {

}


#[test]
fn test_execute_expired_refund_party_b() {

}


#[test]
fn test_execute_expired_no_refund_completes() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.expire_lockup_config();
    suite.tick_contract(suite.holder.clone());
    suite.tick_contract(suite.holder.clone());

    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Complete{}));
}

#[test]
fn test_migrate_update_config() {
    let mut suite = SwapHolderBuilder::default().build();

    let clock_address = suite.query_clock_address();
    let next_contract = suite.query_next_contract();
    let mut parties_config = suite.query_covenant_parties_config();
    parties_config.party_a.native_denom = "new_native_denom".to_string();

    let new_covenant_terms = CovenantTerms::TokenSwap(SwapCovenantTerms {
        party_a_amount: Uint128::zero(),
        party_b_amount: Uint128::one(),
    });
    let new_expiration = Expiration::AtHeight(192837465);

    let resp = suite.app.migrate_contract(
        Addr::unchecked(ADMIN),
        suite.holder.clone(),
        &covenant_swap_holder::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(next_contract.to_string()),
            next_contract: Some(clock_address.to_string()),
            lockup_config: Some(new_expiration.clone()),
            parites_config: Box::new(Some(parties_config.clone())),
            covenant_terms: Some(new_covenant_terms.clone()),
        },
        4,
    )
    .unwrap();

    resp.assert_event(&Event::new("wasm")
        .add_attribute("clock_addr", next_contract.to_string())
        .add_attribute("next_contract", clock_address.to_string())
        .add_attribute("lockup_config", new_expiration.to_string())
        .add_attribute("parites_config", format!("{parties_config:?}"))
        .add_attribute("covenant_terms", format!("{new_covenant_terms:?}"))
    );

    assert_eq!(suite.query_clock_address(), next_contract);
    assert_eq!(suite.query_next_contract(), clock_address);
    assert_eq!(suite.query_contract_state(), ContractState::Instantiated{});
    assert_eq!(suite.query_covenant_parties_config().party_a.native_denom, "new_native_denom");
    assert_eq!(suite.query_covenant_terms(), new_covenant_terms);
}