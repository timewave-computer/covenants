use cosmwasm_std::{coin, coins, Addr, Event, Uint128};
use covenant_utils::{
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    CovenantTerms, SwapCovenantTerms,
};
use cw_multi_test::Executor;
use cw_utils::Expiration;
use valence_swap_holder::msg::{ContractState, RefundConfig};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
};

use super::suite::SwapHolderBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_next_contract() {
    SwapHolderBuilder::default()
        .with_next_contract("invalid_address")
        .build();
}

#[test]
fn test_instantiate_with_valid_op_mode() {
    let _suite = SwapHolderBuilder::default().build();
}

#[test]
fn test_instantiate_in_permissionless_mode() {
    let _suite = SwapHolderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_privileged_accounts() {
    SwapHolderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![
            "some contract".to_string(),
        ]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_empty_privileged_accounts() {
    SwapHolderBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![]))
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
#[should_panic]
fn test_instantiate_validates_party_a_refund_addr() {
    let mut builder = SwapHolderBuilder::default();
    builder
        .instantiate_msg
        .msg
        .refund_config
        .party_a_refund_address = "invalid".to_string();
    builder.build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_party_b_refund_addr() {
    let mut builder = SwapHolderBuilder::default();
    builder
        .instantiate_msg
        .msg
        .refund_config
        .party_b_refund_address = "invalid".to_string();
    builder.build();
}

#[test]
#[should_panic(expected = "Contract operation unauthorized")]
fn test_execute_tick_validates_clock() {
    let mut suite = SwapHolderBuilder::default().build();

    suite
        .app
        .execute_contract(
            suite.admin,
            suite.holder.clone(),
            &valence_swap_holder::msg::ExecuteMsg::Tick {},
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
    assert!(matches!(contract_state, ContractState::Expired {}));
}

#[test]
#[should_panic(expected = "Insufficient funds to forward")]
fn test_execute_tick_instantiated_validates_sufficient_funds() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.tick_contract(suite.holder.clone());
}

#[test]
fn test_execute_tick_instantiated_forwards_and_completes() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.fund_contract(&coins(100000, DENOM_ATOM_ON_NTRN), suite.holder.clone());
    suite.fund_contract(&coins(100000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());

    suite.tick_contract(suite.holder.clone());

    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Complete {}));
}

#[test]
fn test_execute_expired_refund_both_parties() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.fund_contract(&coins(10_000, DENOM_ATOM_ON_NTRN), suite.holder.clone());
    suite.fund_contract(&coins(10_000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());

    suite.assert_balance(suite.holder.clone(), coin(10_000, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(suite.holder.clone(), coin(10_000, DENOM_LS_ATOM_ON_NTRN));

    suite.expire_lockup_config();
    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    suite.tick_contract(suite.holder.clone());
    suite.assert_balance(suite.holder.clone(), coin(0, DENOM_ATOM_ON_NTRN));
    suite.assert_balance(suite.holder.clone(), coin(0, DENOM_LS_ATOM_ON_NTRN));

    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    let refund_config = suite.query_refund_config();
    suite.assert_balance(
        refund_config.party_a_refund_address,
        coin(10_000, DENOM_ATOM_ON_NTRN),
    );
    suite.assert_balance(
        refund_config.party_b_refund_address,
        coin(10_000, DENOM_LS_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_expired_refund_party_a() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.fund_contract(&coins(10_000, DENOM_ATOM_ON_NTRN), suite.holder.clone());
    suite.assert_balance(suite.holder.clone(), coin(10_000, DENOM_ATOM_ON_NTRN));

    suite.expire_lockup_config();
    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    suite.tick_contract(suite.holder.clone());
    suite.assert_balance(suite.holder.clone(), coin(0, DENOM_ATOM_ON_NTRN));

    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    let refund_config = suite.query_refund_config();
    suite.assert_balance(
        refund_config.party_a_refund_address,
        coin(10_000, DENOM_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_expired_refund_party_b() {
    let mut suite = SwapHolderBuilder::default().build();

    suite.fund_contract(&coins(10_000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());
    suite.assert_balance(suite.holder.clone(), coin(10_000, DENOM_LS_ATOM_ON_NTRN));

    suite.expire_lockup_config();
    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    suite.tick_contract(suite.holder.clone());
    suite.assert_balance(suite.holder.clone(), coin(0, DENOM_LS_ATOM_ON_NTRN));

    suite.tick_contract(suite.holder.clone());
    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Expired {}));

    let refund_config = suite.query_refund_config();
    suite.assert_balance(
        refund_config.party_b_refund_address,
        coin(10_000, DENOM_LS_ATOM_ON_NTRN),
    );
}

#[test]
fn test_execute_tick_on_complete_noop() {
    let mut suite = SwapHolderBuilder::default().build();
    suite.fund_contract(&coins(100_000, DENOM_LS_ATOM_ON_NTRN), suite.holder.clone());
    suite.fund_contract(&coins(100_000, DENOM_ATOM_ON_NTRN), suite.holder.clone());

    suite.tick_contract(suite.holder.clone());

    let contract_state = suite.query_contract_state();
    assert!(matches!(contract_state, ContractState::Complete {}));

    suite
        .tick_contract(suite.holder.clone())
        .assert_event(&Event::new("wasm").add_attribute("contract_state", "complete"));
}

#[test]
fn test_migrate_update_config() {
    let mut suite = SwapHolderBuilder::default().build();

    let clock_address = suite.clock_addr.clone();
    let next_contract = suite.query_next_contract();
    let mut parties_config = suite.query_covenant_parties_config();
    parties_config.party_a.native_denom = "new_native_denom".to_string();

    let new_covenant_terms = CovenantTerms::TokenSwap(SwapCovenantTerms {
        party_a_amount: Uint128::zero(),
        party_b_amount: Uint128::one(),
    });
    let new_expiration = Expiration::AtHeight(192837465);
    let new_refund_config = RefundConfig {
        party_a_refund_address: clock_address.to_string(),
        party_b_refund_address: clock_address.to_string(),
    };
    let resp = suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.holder.clone(),
            &valence_swap_holder::msg::MigrateMsg::UpdateConfig {
                op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
                    next_contract.to_string(),
                ])),
                next_contract: Some(clock_address.to_string()),
                lockup_config: Some(new_expiration),
                parties_config: Box::new(Some(parties_config.clone())),
                covenant_terms: Some(new_covenant_terms.clone()),
                refund_config: Some(new_refund_config.clone()),
            },
            4,
        )
        .unwrap();

    let contract_op_mode = ContractOperationMode::Permissioned(vec![next_contract].into());
    resp.assert_event(
        &Event::new("wasm")
            .add_attribute("op_mode", format!("{:?}", contract_op_mode.clone()))
            .add_attribute("next_contract", clock_address.to_string())
            .add_attribute("lockup_config", new_expiration.to_string())
            .add_attribute("parties_config", format!("{parties_config:?}"))
            .add_attribute("covenant_terms", format!("{new_covenant_terms:?}"))
            .add_attribute("refund_config", format!("{new_refund_config:?}")),
    );

    assert_eq!(suite.query_op_mode(), contract_op_mode,);
    assert_eq!(suite.query_next_contract(), clock_address);
    assert_eq!(suite.query_contract_state(), ContractState::Instantiated {});
    assert_eq!(
        suite.query_covenant_parties_config().party_a.native_denom,
        "new_native_denom"
    );
    assert_eq!(suite.query_covenant_terms(), new_covenant_terms);
    assert_eq!(suite.query_refund_config(), new_refund_config);
}

#[test]
#[should_panic(expected = "lockup config is already past")]
fn test_migrate_update_config_validates_lockup_config_expiration() {
    let mut suite = SwapHolderBuilder::default().build();
    suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            suite.holder.clone(),
            &valence_swap_holder::msg::MigrateMsg::UpdateConfig {
                op_mode: None,
                next_contract: None,
                lockup_config: Some(Expiration::AtHeight(1)),
                parties_config: Box::new(None),
                covenant_terms: None,
                refund_config: None,
            },
            4,
        )
        .unwrap();
}
