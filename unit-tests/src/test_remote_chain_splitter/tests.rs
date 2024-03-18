use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, coins, Addr, Decimal, Uint128};
use covenant_remote_chain_splitter::msg::FallbackAddressUpdateConfig;
use covenant_utils::split::SplitConfig;
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_FALLBACK_ON_HUB, DENOM_LS_ATOM_ON_NTRN, DENOM_NTRN,
    DENOM_OSMO_ON_HUB_FROM_NTRN,
};

use super::suite::RemoteChainSplitterBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_clock_address() {
    RemoteChainSplitterBuilder::default()
        .with_clock_address("oo0oOo0".to_string())
        .build();
}

#[test]
#[should_panic(expected = "shares must add up to 1.0")]
fn test_instantiate_validates_explicit_split_shares() {
    let mut builder = RemoteChainSplitterBuilder::default();
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
    let mut split_config = BTreeMap::new();
    split_config.insert("invalid_address".to_string(), Decimal::one());

    let mut invalid_splits = BTreeMap::new();
    invalid_splits.insert(
        DENOM_ATOM_ON_NTRN.to_string(),
        SplitConfig {
            receivers: split_config,
        },
    );

    RemoteChainSplitterBuilder::default()
        .with_splits(invalid_splits)
        .build();
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_execute_tick_validates_clock() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    suite
        .app
        .execute_contract(
            suite.faucet,
            suite.splitter,
            &covenant_remote_chain_splitter::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap();
}

#[test]
fn test_execute_tick_registers_ica() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let splitter = suite.splitter.clone();
    suite.fund_contract(&coins(1000000, DENOM_NTRN), splitter.clone());

    assert!(suite.query_deposit_address(splitter.clone()).is_none());

    suite.tick_contract(splitter.clone());

    assert!(suite.query_deposit_address(splitter.clone()).is_some());
}

#[test]
#[should_panic(expected = "forwarder ica not created not found")]
fn test_execute_tick_split_funds_errors_if_receiver_deposit_address_unavailable() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let splitter = suite.splitter.clone();
    suite.fund_contract(&coins(1000000, DENOM_NTRN), splitter.clone());

    assert!(suite.query_deposit_address(splitter.clone()).is_none());

    suite.tick_contract(splitter.clone());
    suite.tick_contract(splitter);
}

#[test]
fn test_execute_tick_splits_funds_happy() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let splitter = suite.splitter.clone();
    let receiver_1 = suite.receiver_1.clone();
    let receiver_2 = suite.receiver_2.clone();

    suite.fund_contract(&coins(10000000, DENOM_NTRN), splitter.clone());
    suite.fund_contract(&coins(1000000, DENOM_NTRN), receiver_1.clone());
    suite.fund_contract(&coins(1000000, DENOM_NTRN), receiver_2.clone());

    assert!(suite.query_deposit_address(splitter.clone()).is_none());
    assert!(suite.query_deposit_address(receiver_1.clone()).is_none());
    assert!(suite.query_deposit_address(receiver_2.clone()).is_none());

    suite.tick_contract(splitter.clone());
    suite.tick_contract(receiver_1.clone());
    suite.tick_contract(receiver_2.clone());

    let r1_ica = Addr::unchecked(suite.query_deposit_address(receiver_1.clone()).unwrap());
    let r2_ica = Addr::unchecked(suite.query_deposit_address(receiver_2.clone()).unwrap());
    let splitter_ica = Addr::unchecked(suite.query_deposit_address(splitter.clone()).unwrap());

    let zero_bal = coin(0, DENOM_ATOM_ON_NTRN);

    suite.assert_balance(&r1_ica, zero_bal.clone());
    suite.assert_balance(&r2_ica, zero_bal.clone());
    suite.assert_balance(&splitter_ica, zero_bal.clone());

    let amount = coins(10000, DENOM_ATOM_ON_NTRN);
    let amount_halved = coin(5000, DENOM_ATOM_ON_NTRN);

    suite.fund_contract(&amount, splitter_ica.clone());
    suite.assert_balance(&splitter_ica, amount[0].clone());

    suite.tick_contract(splitter);

    suite.assert_balance(&r1_ica, amount_halved.clone());
    suite.assert_balance(&r2_ica, amount_halved.clone());
    suite.assert_balance(&splitter_ica, zero_bal.clone());
}

#[test]
fn test_execute_tick_splits_with_no_leftover() {
    let mut builder = RemoteChainSplitterBuilder::default().with_amount(Uint128::new(100));
    let mut split_config = builder
        .instantiate_msg
        .msg
        .splits
        .get(DENOM_ATOM_ON_NTRN)
        .unwrap()
        .clone();
    let mut first_entry = split_config.receivers.pop_first().unwrap();
    let mut second_entry = split_config.receivers.pop_first().unwrap();

    first_entry.1 = Decimal::from_str("0.107").unwrap();
    second_entry.1 = Decimal::from_str("0.893").unwrap();

    split_config.receivers.insert(first_entry.0, first_entry.1);
    split_config
        .receivers
        .insert(second_entry.0, second_entry.1);

    builder
        .instantiate_msg
        .msg
        .splits
        .insert(DENOM_ATOM_ON_NTRN.to_string(), split_config);

    let mut suite = builder.build();

    let splitter = suite.splitter.clone();
    let receiver_1 = suite.receiver_1.clone();
    let receiver_2 = suite.receiver_2.clone();

    suite.fund_contract(&coins(10000000, DENOM_NTRN), splitter.clone());
    suite.fund_contract(&coins(1000000, DENOM_NTRN), receiver_1.clone());
    suite.fund_contract(&coins(1000000, DENOM_NTRN), receiver_2.clone());

    assert!(suite.query_deposit_address(splitter.clone()).is_none());
    assert!(suite.query_deposit_address(receiver_1.clone()).is_none());
    assert!(suite.query_deposit_address(receiver_2.clone()).is_none());

    suite.tick_contract(splitter.clone());
    suite.tick_contract(receiver_1.clone());
    suite.tick_contract(receiver_2.clone());

    let r1_ica = Addr::unchecked(suite.query_deposit_address(receiver_1.clone()).unwrap());
    let r2_ica = Addr::unchecked(suite.query_deposit_address(receiver_2.clone()).unwrap());
    let splitter_ica = Addr::unchecked(suite.query_deposit_address(splitter.clone()).unwrap());

    let zero_bal = coin(0, DENOM_ATOM_ON_NTRN);

    suite.assert_balance(&r1_ica, zero_bal.clone());
    suite.assert_balance(&r2_ica, zero_bal.clone());
    suite.assert_balance(&splitter_ica, zero_bal.clone());

    let amount = coins(100, DENOM_ATOM_ON_NTRN);
    let expected_first_coin = coin(11, DENOM_ATOM_ON_NTRN);
    let expected_second_coin = coin(89, DENOM_ATOM_ON_NTRN);

    suite.fund_contract(&amount, splitter_ica.clone());
    suite.assert_balance(&splitter_ica, amount[0].clone());

    suite.tick_contract(splitter);

    suite.assert_balance(&r1_ica, expected_first_coin.clone());
    suite.assert_balance(&r2_ica, expected_second_coin.clone());
    suite.assert_balance(&splitter_ica, zero_bal.clone());
}

#[test]
fn test_migrate_update_config() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let mut remote_chain_info = suite.query_remote_chain_info();
    let mut split_config = suite.query_split_config();

    let mut split = split_config.get(DENOM_ATOM_ON_NTRN).unwrap().clone();

    split.receivers.insert(
        suite.receiver_1.to_string(),
        Decimal::from_str("0.1").unwrap(),
    );
    split.receivers.insert(
        suite.receiver_2.to_string(),
        Decimal::from_str("0.9").unwrap(),
    );

    split_config.insert(DENOM_ATOM_ON_NTRN.to_string(), split.clone());

    remote_chain_info.denom = DENOM_LS_ATOM_ON_NTRN.to_string();
    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.splitter.clone(),
            &covenant_remote_chain_splitter::msg::MigrateMsg::UpdateConfig {
                clock_addr: Some(suite.faucet.to_string()),
                remote_chain_info: Some(remote_chain_info.clone()),
                splits: Some(split_config.clone()),
                fallback_address: Some(FallbackAddressUpdateConfig::ExplicitAddress(
                    suite.faucet.to_string(),
                )),
            },
            6,
        )
        .unwrap();

    let new_remote_chain_info = suite.query_remote_chain_info();
    let new_split_config = suite.query_split_config();
    let clock_addr = suite.query_clock_address();
    let fallback_addr = suite.query_fallback_address().unwrap();

    assert_eq!(suite.faucet, clock_addr);
    assert_eq!(remote_chain_info, new_remote_chain_info);
    assert_eq!(split_config, new_split_config);
    assert_eq!(suite.faucet, fallback_addr);
}

#[test]
fn test_migrate_update_config_disable_fallback() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.splitter.clone(),
            &covenant_remote_chain_splitter::msg::MigrateMsg::UpdateConfig {
                clock_addr: None,
                remote_chain_info: None,
                splits: None,
                fallback_address: Some(FallbackAddressUpdateConfig::Disable {}),
            },
            6,
        )
        .unwrap();

    let fallback_addr = suite.query_fallback_address();
    assert!(fallback_addr.is_none());
}

#[test]
#[should_panic(expected = "shares must add up to 1.0")]
fn test_migrate_update_config_validates_splits() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let mut split_config = suite.query_split_config();

    let mut split = split_config.get(DENOM_ATOM_ON_NTRN).unwrap().clone();

    split.receivers.insert(
        suite.receiver_1.to_string(),
        Decimal::from_str("0.41").unwrap(),
    );
    split.receivers.insert(
        suite.receiver_2.to_string(),
        Decimal::from_str("0.9").unwrap(),
    );

    split_config.insert(DENOM_ATOM_ON_NTRN.to_string(), split.clone());

    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.splitter.clone(),
            &covenant_remote_chain_splitter::msg::MigrateMsg::UpdateConfig {
                clock_addr: None,
                remote_chain_info: None,
                splits: Some(split_config.clone()),
                fallback_address: None,
            },
            6,
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Missing fallback address")]
fn test_distribute_fallback_errors_without_fallback_address() {
    let mut suite = RemoteChainSplitterBuilder::default().build();

    let splitter_addr = suite.splitter.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "Cannot distribute target denom via fallback distribution")]
fn test_distribute_fallback_validates_denom() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    let splitter_addr = suite.splitter.clone();

    // fund splitter to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    let splitter_ica = suite.query_ica_address(splitter_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), splitter_ica.clone());
    suite.assert_balance(&splitter_ica, coin(100_000, DENOM_ATOM_ON_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
#[should_panic(expected = "must cover ibc fees to distribute fallback denoms")]
fn test_distribute_fallback_validates_ibc_fee_coverage() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    let splitter_addr = suite.splitter.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    let forwarder_ica = suite.query_ica_address(splitter_addr.clone());

    // fund the ica with sufficient amount of DENOM_ATOM_ON_NTRN
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), forwarder_ica.clone());
    suite.assert_balance(&forwarder_ica, coin(100_000, DENOM_ATOM_ON_NTRN));

    // try to distribute fallback denom
    suite.distribute_fallback(vec![coin(100_000, DENOM_ATOM_ON_NTRN.to_string())], vec![]);
}

#[test]
#[should_panic(expected = "insufficient fees")]
fn test_distribute_fallback_validates_insufficient_ibc_fee_coverage() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    let splitter_addr = suite.splitter.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    let forwarder_ica = suite.query_ica_address(splitter_addr.clone());

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
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    let splitter_addr = suite.splitter.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(2_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    let forwarder_ica = suite.query_ica_address(splitter_addr.clone());

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
#[should_panic(expected = "no ica found")]
fn test_distribute_fallback_validates_ica_exists() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    // try to distribute fallback denom
    suite.distribute_fallback(
        vec![coin(100_000, DENOM_FALLBACK_ON_HUB.to_string())],
        coins(1_000_000, DENOM_NTRN),
    );
}

#[test]
fn test_distribute_fallback_happy() {
    let mut builder = RemoteChainSplitterBuilder::default();
    builder.instantiate_msg.msg.fallback_address =
        Some(builder.instantiate_msg.msg.clock_address.to_string());
    let mut suite = builder.build();

    let splitter_addr = suite.splitter.clone();

    // fund forwarder to register the ica
    suite.fund_contract(&coins(3_000_000, DENOM_NTRN), splitter_addr.clone());

    // register ica
    suite.tick_contract(splitter_addr.clone());

    let forwarder_ica = suite.query_ica_address(splitter_addr.clone());

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
    suite.assert_balance(
        suite.clock_addr.to_string(),
        coin(100_000, DENOM_FALLBACK_ON_HUB),
    );
    suite.assert_balance(
        suite.clock_addr.to_string(),
        coin(100_000, DENOM_OSMO_ON_HUB_FROM_NTRN),
    );
}
