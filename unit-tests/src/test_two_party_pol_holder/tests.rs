use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Decimal, Event};
use covenant_two_party_pol_holder::msg::{ContractState, RagequitTerms, TwoPartyPolCovenantConfig};
use covenant_utils::split::SplitConfig;
use cw_multi_test::Executor;
use cw_utils::Expiration;

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

use super::suite::TwoPartyHolderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_next_contract_addr() {
    TwoPartyHolderBuilder::default()
        .with_next_contract("invalid")
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_clock_addr() {
    TwoPartyHolderBuilder::default()
        .with_clock("invalid")
        .build();
}

#[test]
#[should_panic(expected = "deposit deadline is already past")]
fn test_instantiate_validates_deposit_deadline() {
    TwoPartyHolderBuilder::default()
        .with_deposit_deadline(Expiration::AtHeight(1))
        .build();
}

#[test]
#[should_panic(expected = "lockup deadline is already past")]
fn test_instantiate_validates_lockup_config() {
    TwoPartyHolderBuilder::default()
        .with_lockup_config(Expiration::AtHeight(1))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_covenant_config_router_a_addr() {
    let mut default_builder = TwoPartyHolderBuilder::default();
    default_builder.instantiate_msg.msg.covenant_config.party_a.router = "invalid".to_string();
    default_builder.build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_covenant_config_router_b_addr() {
    let mut default_builder = TwoPartyHolderBuilder::default();
    default_builder.instantiate_msg.msg.covenant_config.party_b.router = "invalid".to_string();
    default_builder.build();
}

#[test]
#[should_panic(expected = "party allocations must add up to 1.0")]
fn test_instantiate_validates_covenant_config_allocations() {
    let mut default_builder = TwoPartyHolderBuilder::default();
    default_builder.instantiate_msg.msg.covenant_config.party_b.allocation = Decimal::from_str("1.1").unwrap();
    default_builder.build();
}


#[test]
#[should_panic(expected = "Ragequit penalty must be in range of [0.0, 1.0)")]
fn test_instantiate_validates_ragequit_config_range() {
    TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("1.1").unwrap(),
            state: None,
        }))
        .build();
}


#[test]
#[should_panic(expected = "Ragequit penalty exceeds party allocation")]
fn test_instantiate_validates_ragequit_config_party_allocations() {
    TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.6").unwrap(),
            state: None,
        }))
        .build();
}

#[test]
// #[should_panic] TODO: enable
fn test_instantiate_validates_explicit_splits() {
    let mut default_builder = TwoPartyHolderBuilder::default();
    let entry: BTreeMap<String, SplitConfig> = default_builder.instantiate_msg.msg.splits
        .iter_mut()
        .map(|(denom, split)| {
            let val = split.receivers.last_key_value().unwrap().0;
            split.receivers.insert(val.clone(), Decimal::from_str("0.6").unwrap());
            (denom.to_string(), split.clone())
        })
        .collect();

    default_builder.instantiate_msg.msg.splits = entry;
    default_builder.build();
}

#[test]
// #[should_panic] TODO
fn test_instantiate_validates_fallback_split() {
    let mut default_builder = TwoPartyHolderBuilder::default();
    let mut fallback_split = SplitConfig {
        receivers: default_builder.instantiate_msg.msg.splits.last_key_value().unwrap().1.receivers.clone(),
    };
    fallback_split.receivers.insert("invalid".to_string(), Decimal::from_str("0.6").unwrap());
    default_builder.instantiate_msg.msg.fallback_split = Some(fallback_split);
    default_builder.build();
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_execute_tick_validates_clock() {
    let mut suite = TwoPartyHolderBuilder::default().build();

    suite.app.execute_contract(
        suite.faucet.clone(),
        suite.holder_addr.clone(),
        &covenant_two_party_pol_holder::msg::ExecuteMsg::Tick {},
        &[],
    )
    .unwrap();
}

#[test]
fn test_execute_tick_expired_deposit_refunds_both_parties() {
    let mut suite = TwoPartyHolderBuilder::default().build();
    suite.expire_deposit_deadline();

    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );

    suite.tick_contract(suite.holder_addr.clone())
        .assert_event(&Event::new("wasm")
            .add_attribute("method", "try_deposit")
            .add_attribute("action", "refund")
        );
    suite.assert_balance(
        &suite.covenant_config.party_a.router,
        coin(10_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        &suite.covenant_config.party_b.router,
        coin(10_000, DENOM_LS_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_tick_expired_deposit_refunds_party_a() {
    let mut suite = TwoPartyHolderBuilder::default().build();
    suite.expire_deposit_deadline();

    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
        ],
        suite.holder_addr.clone(),
    );

    suite.tick_contract(suite.holder_addr.clone())
        .assert_event(&Event::new("wasm")
            .add_attribute("method", "try_deposit")
            .add_attribute("action", "refund")
        );
    suite.assert_balance(
        &suite.covenant_config.party_a.router,
        coin(10_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        &suite.holder_addr.clone(),
        coin(0, DENOM_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_tick_expired_deposit_refunds_party_b() {
    let mut suite = TwoPartyHolderBuilder::default().build();
    suite.app.update_block(|b| b.height = 200000);

    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_LS_ATOM_ON_NTRN),
        ],
        suite.holder_addr.clone(),
    );

    suite.tick_contract(suite.holder_addr.clone())
        .assert_event(&Event::new("wasm")
            .add_attribute("method", "try_deposit")
            .add_attribute("action", "refund")
        );
    suite.assert_balance(
        &suite.covenant_config.party_b.router,
        coin(10_000, DENOM_LS_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        &suite.holder_addr.clone(),
        coin(0, DENOM_LS_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_tick_expired_deposit_completes() {
    let mut suite = TwoPartyHolderBuilder::default().build();
    suite.app.update_block(|b| b.height = 200000);
    suite.tick_contract(suite.holder_addr.clone())
        .assert_event(&Event::new("wasm")
            .add_attribute("method", "try_deposit")
            .add_attribute("state", "complete")
        );
    let state = suite.query_contract_state();
    assert_eq!(state, ContractState::Complete {});
}

#[test]
#[should_panic(expected = "both parties have not deposited")]
fn test_execute_tick_deposit_validates_insufficient_deposits() {
    let mut suite = TwoPartyHolderBuilder::default().build();

    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(5_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());
}

#[test]
fn test_execute_tick_expired_noop() {
    let mut suite = TwoPartyHolderBuilder::default().build();
    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Active {});

    suite.expire_lockup_config();

    suite.tick_contract(suite.holder_addr.clone());
    assert_eq!(suite.query_contract_state(), ContractState::Expired {});

    suite.tick_contract(suite.holder_addr.clone()).assert_event(
        &Event::new("wasm")
            .add_attribute("method", "tick")
            .add_attribute("contract_state", "expired")
        );
}


#[test]
fn test_execute_tick_ragequit_noop() {
    let mut suite = TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.05").unwrap(),
            state: None,
        }))
        .build();
    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());
    suite.tick_contract(suite.next_contract.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Active {});

    suite.expire_deposit_deadline();
    suite.ragequit(&suite.covenant_config.party_a.host_addr.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Ragequit {});

    suite.tick_contract(suite.holder_addr.clone()).assert_event(
        &Event::new("wasm")
            .add_attribute("method", "tick")
            .add_attribute("contract_state", "ragequit")
        );
}

#[test]
#[should_panic(expected = "ragequit is disabled")]
fn test_execute_ragequit_validates_ragequit_config() {
    let mut suite = TwoPartyHolderBuilder::default()
        .build();
    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());
    suite.tick_contract(suite.next_contract.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Active {});

    suite.expire_deposit_deadline();
    suite.ragequit(&suite.covenant_config.party_a.host_addr.clone());
}

#[test]
#[should_panic(expected = "covenant is not in active state")]
fn test_execute_ragequit_validates_active_state() {
    let mut suite = TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.05").unwrap(),
            state: None,
        }))
        .build();

    suite.ragequit(&suite.covenant_config.party_a.host_addr.clone());
}

#[test]
fn test_execute_ragequit_validates_withdraw_started() {
    // todo
}

#[test]
#[should_panic(expected = "covenant is active but expired; tick to proceed")]
fn test_execute_ragequit_validates_lockup_config_expiration() {
    let mut suite = TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.05").unwrap(),
            state: None,
        }))
        .build();
    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());
    suite.tick_contract(suite.next_contract.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Active {});

    suite.expire_lockup_config();
    suite.ragequit(&suite.covenant_config.party_a.host_addr.clone());
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_execute_ragequit_validates_sender() {
    let mut suite = TwoPartyHolderBuilder::default()
        .with_ragequit_config(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.05").unwrap(),
            state: None,
        }))
        .build();
    suite.fund_contract(
        &vec![
            coin(10_000, DENOM_ATOM_ON_NTRN),
            coin(10_000, DENOM_LS_ATOM_ON_NTRN)
        ],
        suite.holder_addr.clone(),
    );
    suite.tick_contract(suite.holder_addr.clone());
    suite.tick_contract(suite.next_contract.clone());

    assert_eq!(suite.query_contract_state(), ContractState::Active {});

    suite.ragequit(&suite.faucet.to_string());
}

#[test]
fn test_execute_ragequit_happy() {

}

