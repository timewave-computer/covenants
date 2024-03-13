use cosmwasm_std::{coin, coins, Uint128};
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_FALLBACK, DENOM_FALLBACK_ON_HUB, DENOM_FALLBACK_ON_OSMO,
    DENOM_HUB_ON_OSMO_FROM_NTRN, DENOM_NTRN, DENOM_NTRN_ON_HUB, DENOM_OSMO,
    DENOM_OSMO_ON_HUB_FROM_NTRN, DENOM_OSMO_ON_NTRN,
};

use super::suite::Suite;

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
            suite.admin.clone(),
            suite.splitter_addr.clone(),
            &covenant_native_splitter::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
        )
        .unwrap();

    // Execute the fallback method on the routers
    suite
        .app
        .execute_contract(
            suite.admin.clone(),
            suite.router_a_addr.clone(),
            &covenant_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
        )
        .unwrap();

    suite
        .app
        .execute_contract(
            suite.admin.clone(),
            suite.router_b_addr.clone(),
            &covenant_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
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
            suite.admin.clone(),
            suite.splitter_addr.clone(),
            &covenant_native_splitter::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
        )
        .unwrap();

    // Execute the fallback method on the routers
    suite
        .app
        .execute_contract(
            suite.admin.clone(),
            suite.router_a_addr.clone(),
            &covenant_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
        )
        .unwrap();

    suite
        .app
        .execute_contract(
            suite.admin.clone(),
            suite.router_b_addr.clone(),
            &covenant_native_router::msg::ExecuteMsg::DistributeFallback {
                denoms: vec![DENOM_FALLBACK.to_string()],
            },
            &[],
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
fn test_covenant_native_refund() {
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
    assert_eq!(receiver_b_balance_ntrn.amount.u128(), 10_000_000_u128);
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
