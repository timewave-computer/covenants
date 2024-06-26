use cosmwasm_std::{coin, coins, Addr, Event};
use covenant_utils::op_mode::{ContractOperationMode, ContractOperationModeConfig};
use cw_multi_test::Executor;

use crate::{
    setup::{
        base_suite::{BaseSuite, BaseSuiteMut},
        ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    },
    test_native_router::suite::NativeRouterBuilder,
};

#[test]
fn test_instantiate_with_valid_op_mode() {
    let _suite = NativeRouterBuilder::default().build();
}

#[test]
fn test_instantiate_in_permissionless_mode() {
    let _suite = NativeRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_privileged_accounts() {
    NativeRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![
            "some contract".to_string(),
        ]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_empty_privileged_accounts() {
    NativeRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![]))
        .build();
}

#[test]
#[should_panic(expected = "Contract operation unauthorized")]
fn test_tick_rejects_unprivileged_account() {
    let mut suite = NativeRouterBuilder::default().build();
    let admin_addr = suite.admin.clone();
    let router_addr = suite.router_addr.clone();
    suite
        .app
        .execute_contract(
            admin_addr,
            router_addr,
            &valence_native_router::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic]
fn test_instantiate_validates_receiver_addr() {
    NativeRouterBuilder::default()
        .with_receiver_address("not a receiver")
        .build();
}

#[test]
fn test_execute_route_balances_with_no_balances() {
    let mut suite = NativeRouterBuilder::default().build();
    let router = suite.router_addr.clone();
    suite.tick_contract(router).assert_event(
        &Event::new("wasm")
            .add_attribute("method", "try_route_balances")
            .add_attribute("balances", "[]"),
    );
}

#[test]
fn test_execute_route_balances_with_one_balance() {
    let mut suite = NativeRouterBuilder::default().build();
    let router = suite.router_addr.clone();

    suite.fund_contract(&coins(5000, DENOM_ATOM_ON_NTRN), router.clone());

    suite.tick_contract(router.clone()).assert_event(
        &Event::new("wasm")
            .add_attribute("method", "try_route_balances")
            .add_attribute(DENOM_ATOM_ON_NTRN.to_string(), "5000"),
    );

    suite.assert_balance(&router, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_addr, coin(5000, DENOM_ATOM_ON_NTRN));
}

#[test]
fn test_execute_route_balances_with_multiple_balances() {
    let mut suite = NativeRouterBuilder::default()
        .with_denoms(vec![
            DENOM_ATOM_ON_NTRN.to_string(),
            DENOM_LS_ATOM_ON_NTRN.to_string(),
        ])
        .build();

    let router = suite.router_addr.clone();

    suite.fund_contract(&coins(5000, DENOM_ATOM_ON_NTRN), router.clone());
    suite.fund_contract(&coins(1000, DENOM_LS_ATOM_ON_NTRN), router.clone());

    suite.tick_contract(router.clone()).assert_event(
        &Event::new("wasm")
            .add_attribute("method", "try_route_balances")
            .add_attribute(DENOM_ATOM_ON_NTRN.to_string(), "5000")
            .add_attribute(DENOM_LS_ATOM_ON_NTRN.to_string(), "1000"),
    );

    suite.assert_balance(&router, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&router, coin(0, DENOM_LS_ATOM_ON_NTRN));

    suite.assert_balance(&suite.receiver_addr, coin(5000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_addr, coin(1000, DENOM_LS_ATOM_ON_NTRN));
}

#[test]
#[should_panic(expected = "unauthorized denom distribution")]
fn test_execute_distribute_fallback_validates_explicit_denoms() {
    let mut suite = NativeRouterBuilder::default().build();

    let router = suite.router_addr.clone();

    suite.fund_contract(&coins(5000, DENOM_ATOM_ON_NTRN), router.clone());
    suite.fund_contract(&coins(1000, DENOM_LS_ATOM_ON_NTRN), router.clone());

    suite.distribute_fallback(vec![
        DENOM_ATOM_ON_NTRN.to_string(),
        DENOM_LS_ATOM_ON_NTRN.to_string(),
    ]);
}

#[test]
fn test_execute_distribute_fallback_happy() {
    let mut suite = NativeRouterBuilder::default().build();

    let router = suite.router_addr.clone();

    suite.fund_contract(&coins(5000, DENOM_ATOM_ON_NTRN), router.clone());
    suite.fund_contract(&coins(1000, DENOM_LS_ATOM_ON_NTRN), router.clone());

    suite
        .distribute_fallback(vec![DENOM_LS_ATOM_ON_NTRN.to_string()])
        .assert_event(&Event::new("wasm").add_attribute("method", "try_distribute_fallback"));

    suite.assert_balance(&router, coin(5000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&router, coin(0, DENOM_LS_ATOM_ON_NTRN));

    suite.assert_balance(&suite.receiver_addr, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_addr, coin(1000, DENOM_LS_ATOM_ON_NTRN));
}

#[test]
fn test_migrate_update_config() {
    let mut suite = NativeRouterBuilder::default().build();

    let router_addr = suite.router_addr.clone();
    let clock_addr = suite.clock_addr.clone();
    let mut target_denoms = suite.query_target_denoms();
    let receiver_addr = suite.receiver_addr.clone();
    target_denoms.insert("new_denom".to_string());

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            router_addr,
            &valence_native_router::msg::MigrateMsg::UpdateConfig {
                op_mode: ContractOperationModeConfig::Permissioned(vec![receiver_addr.to_string()])
                    .into(),
                receiver_address: Some(clock_addr.to_string()),
                target_denoms: Some(target_denoms.clone().into_iter().collect()),
            },
            9,
        )
        .unwrap();

    assert_eq!(
        suite.query_op_mode(),
        ContractOperationMode::Permissioned(vec![receiver_addr].into())
    );
    assert_eq!(suite.query_target_denoms(), target_denoms);
    assert_eq!(suite.query_receiver_config(), clock_addr);
}
