use astroport::{pair::{PoolResponse, SimulationResponse}};
use cosmwasm_std::{Uint128, Addr};

use super::suite::{SuiteBuilder, ST_ATOM_DENOM, NATIVE_ATOM_DENOM};


#[test]
fn test_instantiate_happy() {
    let mut suite = SuiteBuilder::default()
        .build();

    let manual_liq_response = suite.provide_manual_liquidity();
    // println!("\nmanual liquidity response: {:?}\n", manual_liq_response);
    suite.pass_blocks(10);

    // let share_query_resp = suite.query_pool_share();
    // println!("\n1 LP token can withdraw: {:?}\n", share_query_resp);

    // let alice_balances = suite.query_addr_balances(Addr::unchecked("alice"));
    // println!("\n alice balances: \n{:?}\n", alice_balances);

    // let lper_balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.clone().1));
    // println!("\n lper_balances: \n{:?}\n", lper_balances);

    // let pool_balances = suite.query_addr_balances(Addr::unchecked(suite.lp_token.to_string()));
    // println!("\n pool_balances: \n{:?}\n", pool_balances);

    // let res: PoolResponse = suite.query_pool_info();
    // println!("\nQueryMsg::Pool: {:?}\n", res);
    
    let simulation: SimulationResponse = suite.query_simulation();
    println!("\n simulation response: {:?}\n", simulation);
    // suite.tick();


    // suite.mint_coins_to_addr(
    //     suite.liquid_pooler.1.to_string(), 
    //     ST_ATOM_DENOM.to_string(), 
    //     Uint128::new(1000)
    // );
    // suite.mint_coins_to_addr(
    //     suite.liquid_pooler.1.to_string(), 
    //     NATIVE_ATOM_DENOM.to_string(), 
    //     Uint128::new(1000)
    // );

    // let lp_info = suite.query_lp_position();
    // assert_eq!(lp_info.addr, suite.stable_pair.clone().1);

    // suite.pass_blocks(10);

    // // first tick provides liquidity
    // let resp = suite.tick();
    // println!("\n first tick response: {:?}\n\n", resp);

    // let assets = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.clone().1));
    // println!("\nliquid_pooler balances: {:?}\n", assets);

    // suite.pass_blocks(10);

    // let pool_response = suite.query_pool_info();
    // println!("\nQueryMsg::Pool: \n {:?}", pool_response);

    // let alice_balances = suite.query_addr_balances(Addr::unchecked("alice"));
    // println!("\n alice balances: \n{:?}\n", alice_balances);

    // let lper_balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.clone().1));
    // println!("\n lper_balances: \n{:?}\n", lper_balances);

    // let pool_balances = suite.query_addr_balances(Addr::unchecked(suite.lp_token.to_string()));
    // println!("\n pool_balances: \n{:?}\n", pool_balances);

    // let share_query_resp = suite.query_pool_share();
    // println!("\n1 LP token can withdraw: {:?}\n", share_query_resp);

    // let withdrawal_response = suite.withdraw_liquidity(
    //     &Addr::unchecked(CREATOR_ADDR),
    //     1,
    //     vec![],
    // );

    // println!("\n\n withdrawal response: {:?}\n", withdrawal_response);
}

#[test]
fn test_enter_lp() {
    
}