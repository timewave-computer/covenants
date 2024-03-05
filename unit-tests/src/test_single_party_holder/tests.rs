use cosmwasm_std::{coin, coins};

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

use super::suite::{SinglePartyHolderBuilder};


#[test]
fn test_covenant() {
    let suite = SinglePartyHolderBuilder::default().build();
}


#[test]
fn test_instantiate_and_query_withdrawer() {
    let suite = SinglePartyHolderBuilder::default().build();
 
}

// #[test]
// #[should_panic(expected = "Invalid input: address not normalized")]
// fn test_instantiate_invalid_withdrawer() {
//     SinglePartyHolderBuilder::default()
//         .with_withdrawer("0Oo0Oo")
//         .build();

// }

// #[test]
// #[should_panic(expected = "Invalid input: address not normalized")]
// fn test_instantiate_invalid_lp_addr() {
//     SinglePartyHolderBuilder::default()
//         .with_pooler_address("0Oo0Oo")
//         .build();
// }

// #[test]
// #[should_panic(expected = "Unauthorized")]
// fn test_withdraw_all_unauthorized() {
//     let mut suite = SinglePartyHolderBuilder::default().build();

//     suite.fund_contract(coin(100, "coin"), suite.holder_addr.clone());

//     // attacker attempts to withdraw, panic
// }

// #[test]
// fn test_withdraw_all_single_denom() {
//     let mut suite = SinglePartyHolderBuilder::default().build();
//     suite.fund_contract(coin(100, "coin"), suite.holder_addr.clone());


//     // withdraw all

//     // check to see there is no balance

//     // and withdrawer has them all
//  }

#[test]
fn test_withdraw_all_two_denoms() {
    let mut suite = SinglePartyHolderBuilder::default().build();
    suite.fund_contract(&coins(80, DENOM_ATOM_ON_NTRN), suite.holder_addr.clone());
    suite.fund_contract(&coins(70, DENOM_LS_ATOM_ON_NTRN), suite.holder_addr.clone());
    // withdraw all
    
    // assert all funds are now in withdrawer address
}

#[test]
fn test_fund_single_withdraw_partial_single_denom() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.fund_contract(&coins(80, DENOM_ATOM_ON_NTRN), suite.holder_addr.clone());

    // withdraw 75 out of a total of 100 tokens
    
    // check to see there are 25 tokens left in contract
    
    // and holder has received 75
}

#[test]
fn test_fund_multi_denom_withdraw_partial_two_denom() {
    let mut suite = SinglePartyHolderBuilder::default().build();

    suite.fund_contract(&coins(80, DENOM_ATOM_ON_NTRN), suite.holder_addr.clone());
    suite.fund_contract(&coins(70, DENOM_LS_ATOM_ON_NTRN), suite.holder_addr.clone());
}

#[test]
fn test_fund_multi_denom_withdraw_exact_single_denom() {
    let mut suite = SinglePartyHolderBuilder::default().build();

 
    suite.fund_contract(&coins(80, DENOM_ATOM_ON_NTRN), suite.holder_addr.clone());
    suite.fund_contract(&coins(70, DENOM_LS_ATOM_ON_NTRN), suite.holder_addr.clone());
}

// #[test]
// #[should_panic(expected = "Cannot Sub with 70 and 100")]
// fn test_fund_single_and_withdraw_too_big_single_denom() {
//     let mut suite = SinglePartyHolderBuilder::default().build();

//     suite.fund_contract(coin(80, DENOM_ATOM_ON_NTRN), suite.holder_addr.clone());
//     suite.fund_contract(coin(70, DENOM_LS_ATOM_ON_NTRN), suite.holder_addr.clone());
// }

// #[test]
// #[should_panic(expected = "No withdrawer address configured")]
// fn test_withdraw_liquidity_no_withdrawer() {
//     let mut suite = SinglePartyHolderBuilder::default()
//         // .with_withdrawer(None) TODO
//         .build();
// }

// #[test]
// #[should_panic(expected = "No withdrawer address configured")]
// fn test_withdraw_balances_no_withdrawer() {
//     let mut suite = SinglePartyHolderBuilder::default()
//         // .with_withdrawer(None) TODO
//         .build();
// }
