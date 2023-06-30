use cosmwasm_std::{Uint128, Addr};
use cw_multi_test::Executor;

use super::suite::{SuiteBuilder, ST_ATOM_DENOM, NATIVE_ATOM_DENOM, CREATOR_ADDR};


#[test]
fn test_instantiate_happy() {
    let mut suite = SuiteBuilder::default()
        .build();

    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(), 
        ST_ATOM_DENOM.to_string(), 
        Uint128::new(10)
    );
    suite.mint_coins_to_addr(
        suite.liquid_pooler.1.to_string(), 
        NATIVE_ATOM_DENOM.to_string(), 
        Uint128::new(10)
    );

    let resp = suite.app.execute_contract(
        Addr::unchecked(CREATOR_ADDR), 
        Addr::unchecked(suite.liquid_pooler.1),
        &crate::msg::ExecuteMsg::Tick {},
        &[],
    )
    .unwrap();    
}

#[test]
fn test_enter_lp() {
    
}