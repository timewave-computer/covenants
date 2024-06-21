use std::collections::BTreeMap;

use cosmwasm_std::{coin, coins, Addr, Decimal};
use covenant_utils::{
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    split::SplitConfig,
};
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, DENOM_NTRN,
};

use super::suite::NativeSplitterBuilder;

#[test]
#[should_panic(expected = "shares must add up to 1.0")]
fn test_instantiate_validates_explicit_split_shares() {
    let mut builder = NativeSplitterBuilder::default();
    let (denom, mut split_config) = builder.instantiate_msg.msg.splits.pop_first().unwrap();
    let invalid_split_config: BTreeMap<String, Decimal> = split_config
        .receivers
        .iter_mut()
        .map(|(k, _)| (k.to_string(), Decimal::percent(49)))
        .collect();
    builder.instantiate_msg.msg.splits.insert(
        denom,
        SplitConfig {
            receivers: invalid_split_config,
        },
    );
    builder.build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_explicit_split_receiver_addresses() {
    let mut builder = NativeSplitterBuilder::default();
    let (denom, mut split_config) = builder.instantiate_msg.msg.splits.pop_first().unwrap();
    let invalid_split_config: BTreeMap<String, Decimal> = split_config
        .receivers
        .iter_mut()
        .map(|(k, v)| (format!("invalid_{:?}", k), *v))
        .collect();
    builder.instantiate_msg.msg.splits.insert(
        denom,
        SplitConfig {
            receivers: invalid_split_config,
        },
    );
    builder.build();
}

#[test]
fn test_instantiate_with_valid_op_mode() {
    let _suite = NativeSplitterBuilder::default().build();
}

#[test]
fn test_instantiate_in_permissionless_mode() {
    let _suite = NativeSplitterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_privileged_accounts() {
    NativeSplitterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![
            "some contract".to_string(),
        ]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_empty_privileged_accounts() {
    NativeSplitterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_fallback_split_receiver_addresses() {
    let mut invalid_split_config = BTreeMap::new();
    invalid_split_config.insert("invalid_address".to_string(), Decimal::one());
    NativeSplitterBuilder::default()
        .with_fallback_split(Some(SplitConfig {
            receivers: invalid_split_config,
        }))
        .build();
}

#[test]
#[should_panic(expected = "shares must add up to 1.0")]
fn test_instantiate_validates_fallback_split_shares() {
    let builder = NativeSplitterBuilder::default();
    let mut invalid_split_config = BTreeMap::new();
    invalid_split_config.insert(builder.clock_addr.to_string(), Decimal::percent(50));
    builder
        .with_fallback_split(Some(SplitConfig {
            receivers: invalid_split_config,
        }))
        .build();
}

#[test]
#[should_panic(expected = "Contract operation unauthorized")]
fn test_execute_tick_validates_clock() {
    let mut suite = NativeSplitterBuilder::default().build();

    suite
        .app
        .execute_contract(
            suite.faucet,
            suite.splitter,
            &valence_native_splitter::msg::ExecuteMsg::Tick {},
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
            &valence_native_splitter::msg::MigrateMsg::UpdateConfig {
                op_mode: Some(ContractOperationModeConfig::Permissioned(vec![suite
                    .faucet
                    .to_string()])),
                fallback_split: Some(splits.get(DENOM_LS_ATOM_ON_NTRN).unwrap().clone()),
                splits: Some(splits.clone()),
            },
            7,
        )
        .unwrap();

    let op_mode = suite.query_op_mode();
    let new_splits = suite.query_all_splits();
    let new_fallback_split = suite.query_fallback_split();
    let ls_atom_split = suite.query_denom_split(DENOM_LS_ATOM_ON_NTRN.to_string());

    assert!(new_fallback_split.is_some());
    assert_eq!(splits, new_splits);
    assert_eq!(
        op_mode,
        ContractOperationMode::Permissioned(vec![suite.faucet].into())
    );
    assert_eq!(splits.get(DENOM_LS_ATOM_ON_NTRN).unwrap(), &ls_atom_split);
}
