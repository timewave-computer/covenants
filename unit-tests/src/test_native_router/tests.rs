use astroport::router;
use cosmwasm_std::{coin, coins, Addr, Event};
use cw_multi_test::Executor;

use crate::{
    setup::{
        base_suite::{BaseSuite, BaseSuiteMut},
        instantiates::clock,
        ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    },
    test_native_router::suite::NativeRouterBuilder,
};

#[test]
#[should_panic]
fn test_instantiate_validates_clock_addr() {
    NativeRouterBuilder::default()
        .with_clock_address("not a clock")
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_receiver_addr() {
    NativeRouterBuilder::default()
        .with_receiver_address("not a receiver")
        .build();
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_execute_tick_validates_clock_addr() {
    let mut suite = NativeRouterBuilder::default().build();

    let router = suite.router_addr;
    let not_the_clock = suite.faucet;

    suite
        .app
        .execute_contract(
            not_the_clock,
            router,
            &covenant_native_router::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
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
    let clock_addr = suite.query_clock_address();
    let mut target_denoms = suite.query_target_denoms();
    let receiver_addr = suite.receiver_addr.clone();
    target_denoms.insert("new_denom".to_string());

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            router_addr,
            &covenant_native_router::msg::MigrateMsg::UpdateConfig {
                clock_addr: Some(receiver_addr.to_string()),
                receiver_address: Some(clock_addr.to_string()),
                target_denoms: Some(target_denoms.clone().into_iter().collect()),
            },
            9,
        )
        .unwrap();

    assert_eq!(suite.query_clock_address(), receiver_addr);
    assert_eq!(suite.query_target_denoms(), target_denoms);
    assert_eq!(suite.query_receiver_config(), clock_addr);
}
