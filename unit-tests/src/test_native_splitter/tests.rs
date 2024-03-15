use std::collections::BTreeMap;

use cosmwasm_std::{coin, coins, Addr, Decimal};
use covenant_utils::split::SplitConfig;
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, DENOM_NTRN,
};

use super::suite::NativeSplitterBuilder;

#[test]
#[should_panic(expected = "shares must add up to 1.0")]
fn test_instantiate_validates_explicit_splits() {
    let mut split_config = BTreeMap::new();
    split_config.insert("a".to_string(), Decimal::percent(50));
    split_config.insert("b".to_string(), Decimal::percent(60));

    let mut invalid_splits = BTreeMap::new();
    invalid_splits.insert(
        DENOM_ATOM_ON_NTRN.to_string(),
        SplitConfig {
            receivers: split_config,
        },
    );

    NativeSplitterBuilder::default()
        .with_splits(invalid_splits)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_clock_address() {
    NativeSplitterBuilder::default()
        .with_clock_address("invalid_clock".to_string())
        .build();
}

#[test]
fn test_instantiate_validates_fallback_split() {
    let _suite = NativeSplitterBuilder::default().build();
    // should validate
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_execute_tick_validates_clock() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite
        .app
        .execute_contract(
            suite.faucet,
            suite.splitter,
            &covenant_native_splitter::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
}

#[test]
fn test_execute_distribute_single_denom() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.splitter.clone());

    suite.tick_contract(suite.splitter.clone());
    suite.assert_balance(&suite.splitter, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_1, coin(50000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_2, coin(50000, DENOM_ATOM_ON_NTRN));
}

#[test]
fn test_execute_distribute_multiple_denoms() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.splitter.clone());
    suite.fund_contract(
        &coins(100000, DENOM_LS_ATOM_ON_NTRN),
        suite.splitter.clone(),
    );

    suite.tick_contract(suite.splitter.clone());
    suite.assert_balance(&suite.splitter, coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.splitter, coin(0, DENOM_LS_ATOM_ON_NTRN));

    suite.assert_balance(&suite.receiver_1, coin(50000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_1, coin(50000, DENOM_LS_ATOM_ON_NTRN));

    suite.assert_balance(&suite.receiver_2, coin(50000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(&suite.receiver_2, coin(50000, DENOM_LS_ATOM_ON_NTRN));
}

#[test]
#[should_panic(expected = "unauthorized denom distribution")]
fn test_execute_distribute_fallback_validates_explicit_denoms() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.splitter.clone());
    suite.fund_contract(
        &coins(100000, DENOM_LS_ATOM_ON_NTRN),
        suite.splitter.clone(),
    );

    suite.distribute_fallback(vec![DENOM_ATOM_ON_NTRN.to_string()]);
}

#[test]
#[should_panic(expected = "no fallback split defined")]
fn test_execute_distribute_fallback_validates_fallback_split_presence() {
    let mut suite = NativeSplitterBuilder::default()
        .with_fallback_split(None)
        .build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.splitter.clone());
    suite.fund_contract(
        &coins(100000, DENOM_LS_ATOM_ON_NTRN),
        suite.splitter.clone(),
    );

    suite.distribute_fallback(vec![DENOM_ATOM_ON_NTRN.to_string()]);
}

#[test]
fn test_execute_distribute_fallback_happy() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_NTRN), suite.splitter.clone());

    suite.distribute_fallback(vec![DENOM_NTRN.to_string()]);
    suite.assert_balance(&suite.splitter, coin(0, DENOM_NTRN));
    suite.assert_balance(&suite.receiver_1, coin(50000, DENOM_NTRN));
    suite.assert_balance(&suite.receiver_2, coin(50000, DENOM_NTRN));
}

#[test]
fn test_migrate_update_config() {
    let mut suite = NativeSplitterBuilder::default()
        .with_fallback_split(None)
        .build();

    let mut splits = suite.query_all_splits();
    let fallback_split = suite.query_fallback_split();
    assert!(fallback_split.is_none());

    splits.remove(DENOM_ATOM_ON_NTRN);
    assert_eq!(splits.len(), 1);

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.splitter.clone(),
            &covenant_native_splitter::msg::MigrateMsg::UpdateConfig {
                clock_addr: Some(suite.faucet.to_string()),
                fallback_split: Some(splits.get(DENOM_LS_ATOM_ON_NTRN).unwrap().clone()),
                splits: Some(splits.clone()),
            },
            7,
        )
        .unwrap();

    let clock_address = suite.query_clock_address();
    let new_splits = suite.query_all_splits();
    let new_fallback_split = suite.query_fallback_split();
    let ls_atom_split = suite.query_denom_split(DENOM_LS_ATOM_ON_NTRN.to_string());

    assert!(new_fallback_split.is_some());
    assert_eq!(splits, new_splits);
    assert_eq!(clock_address, suite.faucet);
    assert_eq!(splits.get(DENOM_LS_ATOM_ON_NTRN).unwrap(), &ls_atom_split);
}
