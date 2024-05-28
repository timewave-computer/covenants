use cosmwasm_std::{coin, coins, to_json_binary, Addr, Event, Uint128, Uint64};
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    ADMIN, DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_FALLBACK, DENOM_FALLBACK_ON_HUB,
    DENOM_FALLBACK_ON_OSMO, DENOM_HUB_ON_OSMO_FROM_NTRN, DENOM_NTRN, DENOM_NTRN_ON_HUB, DENOM_OSMO,
    DENOM_OSMO_ON_HUB_FROM_NTRN, DENOM_OSMO_ON_NTRN,
};

use super::suite::Suite;

#[test]
#[should_panic(expected = "uatom contribution missing an explicit split configuration")]
fn test_instantiate_validates_split_config_denom_1() {
    Suite::new_with_split_denoms("invalid", DENOM_NTRN);
}

#[test]
#[should_panic(expected = "untrn contribution missing an explicit split configuration")]
fn test_instantiate_validates_split_config_denom_2() {
    Suite::new_with_split_denoms(DENOM_ATOM_ON_NTRN, "invalid");
}

#[test]
fn test_covenant() {
    let mut suite = Suite::new();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_NTRN),
    );

    // tick until holder receive both denoms
    while suite.query_all_balances(&suite.holder_addr).len() < 2 {
        suite.tick("Waiting for holder to receive both denoms");
    }

    // Assert balances are correct and of the correct denoms
    let holder_ntrn_balance = suite.query_balance(&suite.holder_addr, DENOM_NTRN);
    let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM_ON_NTRN);
    assert_eq!(holder_ntrn_balance.amount.u128(), 10_000_000);
    assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

    // Tick until receiver_a gets his split
    while suite.query_all_balances(&suite.party_a_receiver).len() < 2 {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split
    while suite.query_all_balances(&suite.party_b_receiver).len() < 2 {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify receiver_a have 2 denoms in his balances
    let receiver_a_balances = suite.query_all_balances(&suite.party_a_receiver);
    assert_eq!(receiver_a_balances.len(), 2);

    // Verify receiver_b have 2 denoms in his balances
    let receiver_b_balances = suite.query_all_balances(&suite.party_b_receiver);
    assert_eq!(receiver_b_balances.len(), 2);

    // Make sure party_a receiver have the correct denoms
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_ntrn = suite.query_balance(&suite.party_a_receiver, DENOM_NTRN_ON_HUB);
    assert!(receiver_a_balance_ntrn.amount > Uint128::zero());
    assert!(receiver_a_balance_atom.amount > Uint128::zero());

    // make sure party_b receiver have the correct denoms
    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_NTRN);
    let receiver_b_balance_ntrn = suite.query_balance(&suite.party_b_receiver, DENOM_ATOM_ON_NTRN);
    assert!(receiver_b_balance_ntrn.amount > Uint128::zero());
    assert!(receiver_b_balance_atom.amount > Uint128::zero());
}

#[test]
fn test_covenant_2_native_parties() {
    let mut suite = Suite::new_with_2_native_configs();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_NTRN),
    );

    // tick until holder receive both denoms
    while suite.query_all_balances(&suite.holder_addr).len() < 2 {
        suite.tick("Waiting for holder to receive both denoms");
    }

    // Assert balances are correct and of the correct denoms
    let holder_ntrn_balance = suite.query_balance(&suite.holder_addr, DENOM_NTRN);
    let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM);
    assert_eq!(holder_ntrn_balance.amount.u128(), 10_000_000);
    assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

    // Tick until receiver_a gets his split
    while suite.query_all_balances(&suite.party_a_receiver).len() < 2 {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split
    while suite.query_all_balances(&suite.party_b_receiver).len() < 2 {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_ntrn = suite.query_balance(&suite.party_a_receiver, DENOM_NTRN);
    assert!(receiver_a_balance_atom.amount > Uint128::zero());
    assert!(receiver_a_balance_ntrn.amount > Uint128::zero());

    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_ATOM);
    let receiver_b_balance_ntrn = suite.query_balance(&suite.party_b_receiver, DENOM_NTRN);
    assert!(receiver_b_balance_atom.amount > Uint128::zero());
    assert!(receiver_b_balance_ntrn.amount > Uint128::zero());
}

#[test]
fn test_covenant_2_interchain_parties() {
    let mut suite = Suite::new_with_2_interchain_configs();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_OSMO),
    );

    // tick until holder receive both denoms
    while suite.query_all_balances(&suite.holder_addr).len() < 2 {
        suite.tick("Waiting for holder to receive both denoms");
    }

    // Assert balances are correct and of the correct denoms
    let holder_ntrn_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM_ON_NTRN);
    let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_OSMO_ON_NTRN);
    assert_eq!(holder_ntrn_balance.amount.u128(), 10_000_000);
    assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

    // Tick until receiver_a gets his split
    while suite.query_all_balances(&suite.party_a_receiver).len() < 2 {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split
    while suite.query_all_balances(&suite.party_b_receiver).len() < 2 {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_ntrn =
        suite.query_balance(&suite.party_a_receiver, DENOM_OSMO_ON_HUB_FROM_NTRN);
    assert!(receiver_a_balance_atom.amount > Uint128::zero());
    assert!(receiver_a_balance_ntrn.amount > Uint128::zero());

    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_OSMO);
    let receiver_b_balance_ntrn =
        suite.query_balance(&suite.party_b_receiver, DENOM_HUB_ON_OSMO_FROM_NTRN);
    assert!(receiver_b_balance_atom.amount > Uint128::zero());
    assert!(receiver_b_balance_ntrn.amount > Uint128::zero());
}

#[test]
fn test_covenant_100_percent_split() {
    let mut suite = Suite::new_with_100_percent_split();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_OSMO),
    );

    // tick until holder receive both denoms
    while suite.query_all_balances(&suite.holder_addr).len() < 2 {
        suite.tick("Waiting for holder to receive both denoms");
    }

    // Assert balances are correct and of the correct denoms
    let holder_ntrn_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM_ON_NTRN);
    let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_OSMO_ON_NTRN);
    assert_eq!(holder_ntrn_balance.amount.u128(), 10_000_000);
    assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

    // Tick until receiver_a gets his split
    while suite.query_all_balances(&suite.party_a_receiver).is_empty() {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split
    while suite.query_all_balances(&suite.party_b_receiver).is_empty() {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_osmo =
        suite.query_balance(&suite.party_a_receiver, DENOM_OSMO_ON_HUB_FROM_NTRN);
    assert!(receiver_a_balance_atom.amount == Uint128::zero());
    assert!(receiver_a_balance_osmo.amount > Uint128::zero());

    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_OSMO);
    let receiver_b_balance_osmo =
        suite.query_balance(&suite.party_b_receiver, DENOM_HUB_ON_OSMO_FROM_NTRN);
    assert!(receiver_b_balance_atom.amount == Uint128::zero());
    assert!(receiver_b_balance_osmo.amount > Uint128::zero());
}

#[test]
fn test_covenant_fallback_split() {
    let mut suite = Suite::new_with_fallback();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_NTRN),
    );

    // Send some denom (ufallback, not part of the covenant) to the splitter
    suite
        .app
        .send_tokens(
            suite.fuacet.clone(),
            suite.splitter_addr.clone(),
            &coins(1_000_000, DENOM_FALLBACK),
        )
        .unwrap();

    // tick until splitter received all denoms
    // should be 3 because we sent the fallback (ufallback) as well
    while suite.query_all_balances(&suite.splitter_addr).len() < 3 {
        suite.tick("Waiting for splitter to receive both denoms");
    }

    // Execute the fallback method on the splitter
    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.splitter_addr.clone(),
            &valence_native_splitter::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(1000000, DENOM_NTRN)],
        )
        .unwrap();

    // Execute the fallback method on the routers
    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.router_a_addr.clone(),
            &valence_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(1000000, DENOM_NTRN)],
        )
        .unwrap();

    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.router_b_addr.clone(),
            &valence_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(1000000, DENOM_NTRN)],
        )
        .unwrap();

    // Tick until receiver_a gets his split with fallback
    while suite.query_all_balances(&suite.party_a_receiver).len() < 3 {
        println!(
            "party_a balance: {:?}",
            suite.query_all_balances(&suite.party_a_receiver)
        );
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split with fallback
    while suite.query_all_balances(&suite.party_b_receiver).len() < 3 {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_ntrn = suite.query_balance(&suite.party_a_receiver, DENOM_NTRN);
    let receiver_a_balance_fallback = suite.query_balance(&suite.party_a_receiver, DENOM_FALLBACK);
    assert!(receiver_a_balance_atom.amount > Uint128::zero());
    assert!(receiver_a_balance_ntrn.amount > Uint128::zero());
    assert!(receiver_a_balance_fallback.amount > Uint128::zero());

    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_ATOM);
    let receiver_b_balance_ntrn = suite.query_balance(&suite.party_b_receiver, DENOM_NTRN);
    let receiver_b_balance_fallback = suite.query_balance(&suite.party_b_receiver, DENOM_FALLBACK);
    assert!(receiver_b_balance_atom.amount > Uint128::zero());
    assert!(receiver_b_balance_ntrn.amount > Uint128::zero());
    assert!(receiver_b_balance_fallback.amount > Uint128::zero());
}

#[test]
fn test_covenant_interchain_fallback_split() {
    let mut suite = Suite::new_with_interchain_fallback();

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_OSMO),
    );

    // Send some denom (ufallback, not part of the covenant) to the splitter
    suite
        .app
        .send_tokens(
            suite.fuacet.clone(),
            suite.splitter_addr.clone(),
            &coins(1_000_000, DENOM_FALLBACK),
        )
        .unwrap();

    // tick until splitter received all denoms
    // should be 3 because we sent the fallback (ufallback) as well
    while suite.query_all_balances(&suite.splitter_addr).len() < 3 {
        suite.tick("Waiting for splitter to receive all denoms");
    }

    // Execute the fallback method on the splitter
    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.splitter_addr.clone(),
            &valence_interchain_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(1000000, DENOM_NTRN)],
        )
        .unwrap();

    // Execute the fallback method on the routers
    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.router_a_addr.clone(),
            &valence_interchain_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(100000000, DENOM_NTRN)],
        )
        .unwrap();

    suite
        .app
        .execute_contract(
            suite.fuacet.clone(),
            suite.router_b_addr.clone(),
            &valence_interchain_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[coin(100000000, DENOM_NTRN)],
        )
        .unwrap();

    // Tick until receiver_a gets his split with fallback
    while suite.query_all_balances(&suite.party_a_receiver).len() < 3 {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split with fallback
    while suite.query_all_balances(&suite.party_b_receiver).len() < 3 {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    let receiver_a_balance_osmo =
        suite.query_balance(&suite.party_a_receiver, DENOM_OSMO_ON_HUB_FROM_NTRN);
    let receiver_a_balance_fallback =
        suite.query_balance(&suite.party_a_receiver, DENOM_FALLBACK_ON_HUB);
    assert!(receiver_a_balance_atom.amount > Uint128::zero());
    assert!(receiver_a_balance_osmo.amount > Uint128::zero());
    assert!(receiver_a_balance_fallback.amount > Uint128::zero());

    let receiver_b_balance_atom = suite.query_balance(&suite.party_b_receiver, DENOM_OSMO);
    let receiver_b_balance_osmo =
        suite.query_balance(&suite.party_b_receiver, DENOM_HUB_ON_OSMO_FROM_NTRN);
    let receiver_b_balance_fallback =
        suite.query_balance(&suite.party_b_receiver, DENOM_FALLBACK_ON_OSMO);
    assert!(receiver_b_balance_atom.amount > Uint128::zero());
    assert!(receiver_b_balance_osmo.amount > Uint128::zero());
    assert!(receiver_b_balance_fallback.amount > Uint128::zero());
}

#[test]
fn test_valence_native_refund() {
    let mut suite = Suite::new_with_2_native_configs();
    let init_ntrn_router_balance = suite.query_balance(&suite.router_b_addr, DENOM_NTRN);

    // Wait until depositors are ready and fund them
    suite.get_and_fund_depositors(
        coin(10_000_000_u128, DENOM_ATOM),
        coin(10_000_000_u128, DENOM_NTRN),
    );

    // tick until holder receive both denoms
    while suite.query_all_balances(&suite.holder_addr).len() < 2 {
        suite.tick("Waiting for holder to receive both denoms");
    }

    // Assert balances are correct and of the correct denoms
    let holder_ntrn_balance = suite.query_balance(&suite.holder_addr, DENOM_NTRN);
    let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM);
    assert_eq!(holder_ntrn_balance.amount.u128(), 10_000_000);
    assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

    // Expire the covenant
    suite.app.update_block(|block| {
        block.time = block.time.plus_hours(1_000_000);
        block.height += 1_000_000;
    });
    // tick to trigger the expiration
    suite.tick_contract(suite.holder_addr.clone());

    // Tick until receiver_a gets his split
    while suite.query_all_balances(&suite.party_a_receiver).is_empty() {
        suite.tick("Wait for receiver_a to get his split");
    }

    // Tick until receiver_b gets his split
    while suite.query_all_balances(&suite.party_b_receiver).is_empty() {
        suite.tick("Wait for receiver_b to get his split");
    }

    // Verify balances of receivers are correct
    let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
    assert_eq!(receiver_a_balance_atom.amount.u128(), 10_000_000_u128);

    let receiver_b_balance_ntrn = suite.query_balance(&suite.party_b_receiver, DENOM_NTRN);
    // router comes prefunded with some ntrn so we add that to the assertion
    assert_eq!(
        receiver_b_balance_ntrn.amount.u128(),
        10_000_000_u128 + init_ntrn_router_balance.amount.u128()
    );
}

#[test]
fn test_migrate_update_with_codes() {
    let mut suite = Suite::new_with_2_native_configs();
    let covenant_addr = suite.covenant_addr.to_string();

    let mut contract_codes = suite.query_contract_codes();
    contract_codes.clock = 1;

    let native_router_migrate_msg = valence_native_router::msg::MigrateMsg::UpdateConfig {
        privileged_accounts: Some(vec![covenant_addr.to_string()].into()),
        target_denoms: None,
        receiver_address: None,
    };

    let holder_migrate_msg = valence_swap_holder::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(covenant_addr.to_string()),
        next_contract: None,
        lockup_config: None,
        parites_config: Box::new(None),
        covenant_terms: None,
        refund_config: None,
    };

    let splitter_migrate_msg = valence_native_splitter::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(covenant_addr.to_string()),
        fallback_split: None,
        splits: None,
    };

    let resp = suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            Addr::unchecked(covenant_addr),
            &valence_covenant_swap::msg::MigrateMsg::UpdateCovenant {
                codes: Some(contract_codes.clone()),
                clock: None,
                holder: Some(holder_migrate_msg.clone()),
                splitter: Some(splitter_migrate_msg.clone()),
                party_a_router: Some(valence_covenant_swap::msg::RouterMigrateMsg::Native(
                    native_router_migrate_msg.clone(),
                )),
                party_b_router: Some(valence_covenant_swap::msg::RouterMigrateMsg::Native(
                    native_router_migrate_msg.clone(),
                )),
                party_a_forwarder: Box::new(None),
                party_b_forwarder: Box::new(None),
            },
            1,
        )
        .unwrap();

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute(
                "contract_codes_migrate",
                to_json_binary(&contract_codes).unwrap().to_base64(),
            )
            .add_attribute(
                "holder_migrate",
                to_json_binary(&holder_migrate_msg).unwrap().to_base64(),
            )
            .add_attribute(
                "splitter_migrate",
                to_json_binary(&splitter_migrate_msg).unwrap().to_base64(),
            )
            .add_attribute(
                "party_a_router_migrate",
                to_json_binary(&native_router_migrate_msg)
                    .unwrap()
                    .to_base64(),
            )
            .add_attribute(
                "party_b_router_migrate",
                to_json_binary(&native_router_migrate_msg)
                    .unwrap()
                    .to_base64(),
            ),
    );

    let new_codes = suite.query_contract_codes();
    assert_eq!(contract_codes, new_codes);
}

#[test]
fn test_migrate_update_without_codes() {
    let mut suite = Suite::new_with_2_interchain_configs();
    let covenant_addr = suite.covenant_addr.to_string();

    let interchain_router_migrate_msg = valence_interchain_router::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(covenant_addr.to_string()),
        target_denoms: None,
        destination_config: None,
    };

    let ibc_forwarder_migrate_msg = valence_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
        privileged_accounts: Some(vec![covenant_addr.to_string()].into()),
        next_contract: None,
        remote_chain_info: Box::new(None),
        transfer_amount: None,
        fallback_address: None,
    };

    let clock_migrate_msg = valence_clock::msg::MigrateMsg::UpdateTickMaxGas {
        new_value: Uint64::new(50000),
    };

    let resp = suite
        .app
        .migrate_contract(
            Addr::unchecked(ADMIN),
            Addr::unchecked(covenant_addr),
            &valence_covenant_swap::msg::MigrateMsg::UpdateCovenant {
                codes: None,
                clock: Some(clock_migrate_msg.clone()),
                holder: None,
                splitter: None,
                party_a_router: Some(valence_covenant_swap::msg::RouterMigrateMsg::Interchain(
                    interchain_router_migrate_msg.clone(),
                )),
                party_b_router: Some(valence_covenant_swap::msg::RouterMigrateMsg::Interchain(
                    interchain_router_migrate_msg.clone(),
                )),
                party_a_forwarder: Box::new(Some(ibc_forwarder_migrate_msg.clone())),
                party_b_forwarder: Box::new(Some(ibc_forwarder_migrate_msg.clone())),
            },
            1,
        )
        .unwrap();

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute(
                "clock_migrate",
                to_json_binary(&clock_migrate_msg).unwrap().to_base64(),
            )
            .add_attribute(
                "party_a_router_migrate",
                to_json_binary(&interchain_router_migrate_msg)
                    .unwrap()
                    .to_base64(),
            )
            .add_attribute(
                "party_b_router_migrate",
                to_json_binary(&interchain_router_migrate_msg)
                    .unwrap()
                    .to_base64(),
            )
            .add_attribute(
                "party_a_forwarder_migrate",
                to_json_binary(&ibc_forwarder_migrate_msg)
                    .unwrap()
                    .to_base64(),
            )
            .add_attribute(
                "party_b_forwarder_migrate",
                to_json_binary(&ibc_forwarder_migrate_msg)
                    .unwrap()
                    .to_base64(),
            ),
    );
}

// TODO: swap holder is using IBC transfer method isntead of the neutron msg, so this test is not working
// #[test]
// fn test_covenant_interchain_refund() {
//     let mut suite = Suite::new_with_2_interchain_configs();

//     // Wait until depositors are ready and fund them
//     suite.get_and_fund_depositors(
//         coin(10_000_000_u128.into(), DENOM_ATOM),
//         coin(10_000_000_u128.into(), DENOM_OSMO),
//     );

//     // tick until holder receive both denoms
//     while suite.query_all_balances(&suite.holder_addr).len() < 2 {
//         suite.tick("Waiting for holder to receive both denoms");
//     }

//     // Assert balances are correct and of the correct denoms
//     let holder_osmo_balance = suite.query_balance(&suite.holder_addr, DENOM_ATOM_ON_NTRN);
//     let holder_atom_balance = suite.query_balance(&suite.holder_addr, DENOM_OSMO_ON_NTRN);
//     assert_eq!(holder_osmo_balance.amount.u128(), 10_000_000);
//     assert_eq!(holder_atom_balance.amount.u128(), 10_000_000);

//     // Expire the covenant
//     suite.app.update_block(|block| {
//         block.time = block.time.plus_hours(1_000_000);
//         block.height = block.height + 1_000_000;
//     });

//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     let res = suite.tick("Wait for receiver_a to get his refund");
//     println!("res: {:?}", res);

//     return;
//     // Tick until receiver_a gets his refund
//     while suite.query_all_balances(&suite.party_a_receiver).len() < 1 {
//         suite.tick("Wait for receiver_a to get his refund");
//         println!(
//             "holder balance: {:?}",
//             suite.query_all_balances(&suite.holder_addr)
//         );
//     }

//     // Tick until receiver_b gets his refund
//     while suite.query_all_balances(&suite.party_b_receiver).len() < 1 {
//         suite.tick("Wait for receiver_b to get his refund");
//     }

//     // Verify balances of receivers are correct
//     let receiver_a_balance_atom = suite.query_balance(&suite.party_a_receiver, DENOM_ATOM);
//     assert_eq!(receiver_a_balance_atom.amount.u128(), 10_000_000_u128);

//     let receiver_b_balance_osmo = suite.query_balance(&suite.party_b_receiver, DENOM_OSMO);
//     assert_eq!(receiver_b_balance_osmo.amount.u128(), 10_000_000_u128);
// }
