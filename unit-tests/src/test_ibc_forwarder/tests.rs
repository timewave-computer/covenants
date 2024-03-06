use cosmwasm_std::{coin, coins, Addr, Uint128};
use covenant_ibc_forwarder::msg::ContractState;
use cw_multi_test::Executor;

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, instantiates::clock, ADMIN, DENOM_ATOM_ON_NTRN, DENOM_NTRN};

use super::suite::IbcForwarderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_next_contract_addr() {
    IbcForwarderBuilder::default()
        .with_next_contract("some contract".to_string())
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_clock_addr() {
    IbcForwarderBuilder::default()
        .with_clock_address("some contract".to_string())
        .build();
}

#[test]
#[should_panic(expected = "not the clock")]
fn test_tick_validates_clock() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.app.execute_contract(
        forwarder_addr.clone(),
        forwarder_addr.clone(),
        &covenant_ibc_forwarder::msg::ExecuteMsg::Tick {},
        &[],
    )
    .unwrap();
}


#[test]
#[should_panic(expected = "Cannot Sub with 0 and 1000000")]
fn test_ica_registration_takes_fee() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.tick_contract(forwarder_addr);
}

#[test]
fn test_ica_registration() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), forwarder_addr.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Instantiated {});

    suite.tick_contract(forwarder_addr.clone());

    let ica_addr = suite.query_ica_address(forwarder_addr);
    assert!(!ica_addr.to_string().is_empty());
    assert_eq!(suite.query_contract_state(), ContractState::IcaCreated {});
}

#[test]
#[should_panic]
fn test_forward_funds_next_contract_does_not_implement_deposit_address_query() {
    let mut suite = IbcForwarderBuilder::default()
        .with_next_contract("cosmos10a6yf8khw53pvmafngsq2vjgqgu3p9kjsgpzpa2vm9ceg0c70eysqg42pu".to_string())
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), forwarder_addr.clone());
    
    // register ica
    suite.tick_contract(forwarder_addr.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);
}

#[test]
#[should_panic(expected = "Next contract is not ready for receiving the funds yet")]
fn test_forward_funds_next_contract_not_ready() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);
}

#[test]
fn test_forward_funds_insufficient() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    let next_contract = suite.query_next_contract();


    // fund both contracts to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), next_contract.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());
    suite.tick_contract(next_contract.clone());
    
    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with insufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(
        &coins(99_000, DENOM_ATOM_ON_NTRN),
        forwarder_ica.clone(),
    );

    // try to forward
    suite.tick_contract(forwarder_addr);

    // assert that the funds were not forwarded
    suite.assert_balance(&forwarder_ica, coin(99_000, DENOM_ATOM_ON_NTRN));
}

#[test]
fn test_forward_funds_happy() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    let next_contract = suite.query_next_contract();


    // fund both contracts to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), next_contract.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());
    suite.tick_contract(next_contract.clone());
    
    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());
    let next_contract_deposit_addr = suite.query_ica_address(next_contract.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(
        &coins(100_000, DENOM_ATOM_ON_NTRN),
        forwarder_ica.clone(),
    );

    // try to forward
    suite.tick_contract(forwarder_addr);

    // assert that the funds were in fact forwarded
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_ATOM_ON_NTRN));
    // hacky ibc denom assertion
    suite.assert_balance(&next_contract_deposit_addr, coin(100_000, "channel-1/channel-1/uatom"));
}

#[test]
fn test_migrate_update_config() {
    let mut suite = IbcForwarderBuilder::default()
        .build();
    
    let forwarder_addr = suite.ibc_forwarder.clone();
    let next_contract = suite.query_next_contract();
    let mut remote_chain_info = suite.query_remote_chain_info();
    remote_chain_info.denom = "some new denom".to_string();
    let clock_addr = suite.query_clock_address();

    // migrate
    suite.app.migrate_contract(
        Addr::unchecked(ADMIN),
        forwarder_addr.clone(),
        &covenant_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(next_contract.to_string()),
            next_contract: Some(clock_addr.to_string()),
            remote_chain_info: Box::new(Some(remote_chain_info)),
            transfer_amount: Some(Uint128::new(69)),
        },
        10,
    )
    .unwrap();

    assert_eq!(suite.query_clock_address(), next_contract);
    assert_eq!(suite.query_remote_chain_info().denom, "some new denom");
    assert_eq!(suite.query_transfer_amount(), Uint128::new(69));
    assert_eq!(suite.query_next_contract(), clock_addr);
}