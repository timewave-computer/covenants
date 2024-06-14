use cosmwasm_std::{coin, coins, Addr, Uint128};
use covenant_utils::op_mode::{ContractOperationMode, ContractOperationModeConfig};
use cw_multi_test::Executor;
use valence_ibc_forwarder::msg::{ContractState, FallbackAddressUpdateConfig};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_FALLBACK_ON_HUB, DENOM_NTRN, DENOM_OSMO_ON_HUB_FROM_NTRN,
};

use super::suite::IbcForwarderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_next_contract_addr() {
    IbcForwarderBuilder::default()
        .with_next_contract("some contract".to_string())
        .build();
}

#[test]
fn test_instantiate_with_valid_op_mode() {
    let _suite = IbcForwarderBuilder::default().build();
}

#[test]
fn test_instantiate_in_permissionless_mode() {
    let _suite = IbcForwarderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_privileged_accounts() {
    IbcForwarderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![
            "some contract".to_string(),
        ]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_empty_privileged_accounts() {
    IbcForwarderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![]))
        .build();
}

#[test]
#[should_panic(expected = "Contract operation unauthorized")]
fn test_tick_rejects_unprivileged_account() {
    let mut suite = IbcForwarderBuilder::default().build();
    let admin_addr = suite.admin.clone();
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite
        .app
        .execute_contract(
            admin_addr,
            forwarder_addr,
            &valence_ibc_forwarder::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Cannot Sub with 0 and 1000000")]
fn test_ica_registration_takes_fee() {
    let mut suite = IbcForwarderBuilder::default().build();
    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.tick_contract(forwarder_addr);
}

#[test]
fn test_ica_registration() {
    let mut suite = IbcForwarderBuilder::default().build();

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
        .with_next_contract(
            "cosmos10a6yf8khw53pvmafngsq2vjgqgu3p9kjsgpzpa2vm9ceg0c70eysqg42pu".to_string(),
        )
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
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();
    suite.fund_contract(&coins(1_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);
}

#[test]
fn test_forward_funds_insufficient() {
    let mut suite = IbcForwarderBuilder::default().build();

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
    suite.fund_contract(&coins(99_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);

    // assert that the funds were not forwarded
    suite.assert_balance(&forwarder_ica, coin(99_000, DENOM_ATOM_ON_NTRN));
}

#[test]
fn test_forward_funds_happy_permissioned() {
    let mut suite = IbcForwarderBuilder::default().build();

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
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);

    // assert that the funds were in fact forwarded
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_ATOM_ON_NTRN));
    // hacky ibc denom assertion
    suite.assert_balance(
        next_contract_deposit_addr,
        coin(100_000, "channel-1/channel-1/uatom"),
    );
}

#[test]
fn test_forward_funds_happy_permissionless() {
    let mut suite = IbcForwarderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
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
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());

    // try to forward
    suite.tick_contract(forwarder_addr);

    // assert that the funds were in fact forwarded
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_ATOM_ON_NTRN));
    // hacky ibc denom assertion
    suite.assert_balance(
        next_contract_deposit_addr,
        coin(100_000, "channel-1/channel-1/uatom"),
    );
}

#[test]
#[should_panic(expected = "Missing fallback address")]
fn test_distribute_fallback_errors_without_fallback_address() {
    let mut builder = IbcForwarderBuilder::default();
    builder.instantiate_msg.msg.fallback_address = None;
    let mut suite = builder.build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "Cannot distribute target denom via fallback distribution")]
fn test_distribute_fallback_validates_denom() {
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_ATOM_ON_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "must cover ibc fees to distribute fallback denoms")]
fn test_distribute_fallback_validates_ibc_fee_coverage() {
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_ATOM_ON_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())], vec![]);
}

#[test]
#[should_panic(expected = "no ica found")]
fn test_distribute_fallback_validates_ica_exists() {
    let mut suite = IbcForwarderBuilder::default().build();

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_FALLBACK_ON_HUB.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "insufficient fees")]
fn test_distribute_fallback_validates_insufficient_ibc_fee_coverage() {
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_ATOM_ON_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())],
        coins(5_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "Attempt to distribute duplicate denoms via fallback distribution")]
fn test_distribute_fallback_validates_duplicate_input_denoms() {
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(
        &coins(100_000, DENOM_FALLBACK_ON_HUB),
        forwarder_ica.clone(),
    );
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_FALLBACK_ON_HUB));

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![
            coin(100_000, DENOM_FALLBACK_ON_HUB.to_string()),
            coin(100_000, DENOM_FALLBACK_ON_HUB.to_string()),
        ],
        coins(1_000_000, DENOM_NTRN),
    );

    // assert that the funds were in fact forwarded
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_FALLBACK_ON_HUB));
}

#[test]
fn test_distribute_fallback_happy() {
    let mut suite = IbcForwarderBuilder::default().build();

    let forwarder_addr = suite.ibc_forwarder.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(3_000_000, DENOM_NTRN), forwarder_addr.clone());

    // register ica
    suite.tick_contract(forwarder_addr.clone());

    let forwarder_ica = suite.query_ica_address(forwarder_addr.clone());

    // fund the ica with sufficient amount of DENOM_FALLBACK_ON_HUB and
    suite.fund_contract(
        &coins(100_000, DENOM_FALLBACK_ON_HUB),
        forwarder_ica.clone(),
    );
    suite.fund_contract(
        &coins(100_000, DENOM_OSMO_ON_HUB_FROM_NTRN),
        forwarder_ica.clone(),
    );
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_FALLBACK_ON_HUB));
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_OSMO_ON_HUB_FROM_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![
            coin(100_000, DENOM_FALLBACK_ON_HUB.to_string()),
            coin(100_000, DENOM_OSMO_ON_HUB_FROM_NTRN),
        ],
        vec![coin(2_000_000, DENOM_NTRN)],
    );

    // assert that the funds were in fact forwarded
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&forwarder_ica, coin(0, DENOM_OSMO_ON_HUB_FROM_NTRN));

    let fallback_addr = suite.query_fallback_address().unwrap();

    suite.assert_balance(fallback_addr.clone(), coin(100_000, DENOM_FALLBACK_ON_HUB));
    suite.assert_balance(fallback_addr, coin(100_000, DENOM_OSMO_ON_HUB_FROM_NTRN));
}

#[test]
fn test_migrate_update_config() {
    let mut ibc_fwdr_builder = IbcForwarderBuilder::default();

    let upd_next_contract = ibc_fwdr_builder.builder.get_random_addr().to_string();
    let upd_priv_addresses = vec![ibc_fwdr_builder.builder.get_random_addr()];
    let upd_fallback_address = ibc_fwdr_builder.builder.get_random_addr().to_string();

    let mut suite = ibc_fwdr_builder.build();

    let forwarder_addr = suite.ibc_forwarder.clone();
    let mut remote_chain_info = suite.query_remote_chain_info();
    remote_chain_info.denom = "some new denom".to_string();

    // migrate
    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            forwarder_addr.clone(),
            &valence_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
                op_mode: Some(ContractOperationModeConfig::Permissioned(
                    upd_priv_addresses.iter().map(|a| a.to_string()).collect(),
                )),
                next_contract: Some(upd_next_contract.to_string()),
                remote_chain_info: Box::new(Some(remote_chain_info)),
                transfer_amount: Some(Uint128::new(69)),
                fallback_address: Some(FallbackAddressUpdateConfig::ExplicitAddress(
                    upd_fallback_address.clone(),
                )),
            },
            10,
        )
        .unwrap();

    assert_eq!(
        suite.query_op_mode(),
        ContractOperationMode::Permissioned(upd_priv_addresses.into())
    );
    assert_eq!(suite.query_remote_chain_info().denom, "some new denom");
    assert_eq!(suite.query_transfer_amount(), Uint128::new(69));
    assert_eq!(suite.query_next_contract(), upd_next_contract);
    assert_eq!(
        suite.query_fallback_address().unwrap(),
        upd_fallback_address
    );
}

#[test]
fn test_migrate_update_config_remove_fallback() {
    let mut suite = IbcForwarderBuilder::default().build();

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.ibc_forwarder.clone(),
            &valence_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
                op_mode: None,
                next_contract: None,
                remote_chain_info: Box::new(None),
                transfer_amount: None,
                fallback_address: Some(FallbackAddressUpdateConfig::Disable {}),
            },
            10,
        )
        .unwrap();

    assert!(suite.query_fallback_address().is_none());
}
