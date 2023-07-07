
use cosmwasm_std::{Uint128, Addr, Decimal, };


use super::suite::{SuiteBuilder, ST_ATOM_DENOM, NATIVE_ATOM_DENOM};


#[test]
fn test_instantiate_happy() {
    let mut suite = SuiteBuilder::default()
        .build();

    let redemption_rate = Decimal::from_ratio(Uint128::new(12), Uint128::new(10));
    let atom_amt = Uint128::new(400000);
    let statom_amt = atom_amt * redemption_rate;
    // fund pool with balanced amounts of underlying tokens
    suite.provide_manual_liquidity("alice".to_string(), statom_amt, atom_amt);
    
    // fund LP contract with some tokens to provide liquidity with
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(), 
        ST_ATOM_DENOM.to_string(), 
        Uint128::new(100000)
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(), 
        NATIVE_ATOM_DENOM.to_string(), 
        Uint128::new(100000)
    );


    let liquid_pooler_balances = suite.query_addr_balances(
        Addr::unchecked(suite.liquid_pooler.1.to_string())
    );
    assert_eq!(liquid_pooler_balances[0].amount, Uint128::new(100000));
    assert_eq!(liquid_pooler_balances[1].amount, Uint128::new(100000));
    let share = suite.query_pool_info();
    println!("pool share: {:?}", share);

    let pairinfo: astroport::asset::PairInfo = suite.app.wrap().query_wasm_smart(
        suite.stable_pair.1.to_string(),
        &astroport::pair::QueryMsg::Pair { }
    ).unwrap();
    let liquidity_token_addr = pairinfo.liquidity_token;

    suite.tick();

    suite.pass_blocks(10);
    let share = suite.query_pool_info();
    println!("pool share: {:?}", share);
    let liquid_pooler_balances = suite.query_addr_balances(
        Addr::unchecked(suite.liquid_pooler.1.to_string())
    );
    assert!(liquid_pooler_balances.is_empty());
    
    suite.withdraw();
    suite.pass_blocks(10);

    let liquid_pooler_balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    println!("\n post withdrawal liquid pooler balances: {:?}\n", liquid_pooler_balances);
}

#[test]
fn test_malicious_lp_funding() {
    
}

#[test]
fn test_lp_empty() {

}

#[test]
fn test_lp_perfectly_balanced() {

}

#[test]
fn test_lp_first_asset_dominant() {

}

#[test]
fn test_lp_second_asset_dominant() {

}

#[test]
fn test_lp_extreme_imbalance() {

}

#[test]
fn test_withdraw_unauthorized() {

}
