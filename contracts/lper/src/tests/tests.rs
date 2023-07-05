use astroport::{pair::{PoolResponse, SimulationResponse, Cw20HookMsg}, asset::{AssetInfo, PairInfo, Asset}};
use cosmwasm_std::{Uint128, Addr, Coin, to_binary};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, AllowanceResponse, AllAllowancesResponse};
use cw_multi_test::Executor;
use neutron_sdk::bindings::msg::MsgExecuteContract;

use crate::tests::suite::CREATOR_ADDR;

use super::suite::{SuiteBuilder, ST_ATOM_DENOM, NATIVE_ATOM_DENOM};


#[test]
fn test_instantiate_happy() {
    let mut suite = SuiteBuilder::default()
        .build();

    // suite.provide_manual_liquidity("alice".to_string());
    
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


    // let liquid_pooler_balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    // assert_eq!(liquid_pooler_balances.len(), 2);
    // println!("\n liquid pooler balances: {:?}\n", liquid_pooler_balances);

    // let stable_pair_balances = suite.query_addr_balances(Addr::unchecked(suite.stable_pair.1.to_string()));
    // println!("\n stable_pair_balances: {:?}\n", stable_pair_balances);

    let query_pair_token: PairInfo = suite.app.wrap().query_wasm_smart(
        Addr::unchecked(suite.factory.1.to_string()),
        &astroport::factory::QueryMsg::Pair { asset_infos: vec![
            AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() },
            AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() },
        ]}
    ).unwrap();
    println!("\n factory token address query: {:?}\n", query_pair_token);

    println!("\n contract addr: {:?}", query_pair_token.contract_addr);

    let liq_msg: cw_multi_test::AppResponse = suite.app.execute_contract(
        Addr::unchecked(suite.liquid_pooler.1.to_string()),
        Addr::unchecked(suite.stable_pair.1.to_string()),
        &astroport::pair::ExecuteMsg::ProvideLiquidity { 
            assets: vec![
                Asset { 
                    info: AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() }, 
                    amount: Uint128::new(5000),
                },
                Asset { 
                    info: AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() }, 
                    amount: Uint128::new(5000),
                },
            ],
            slippage_tolerance: None,
            auto_stake: Some(false),
            receiver: Some(suite.liquid_pooler.1.to_string()),
        },
        &[
            Coin { 
                denom: ST_ATOM_DENOM.to_string(), 
                amount: Uint128::new(5000),
            },
            Coin { 
                denom: NATIVE_ATOM_DENOM.to_string(), 
                amount: Uint128::new(5000),
            },
        ]
    ).unwrap();
    
    // println!("\nliq response: {:?}\n", liq_msg);

    // suite.app.execute_contract(
    //     Addr::unchecked(suite.stable_pair.1.to_string()),
    //     Addr::unchecked("contract6".to_string()),
    //     &Cw20ExecuteMsg::IncreaseAllowance {
    //         spender: query_pair_token.contract_addr.to_string(),
    //         amount: Uint128::new(1000000),
    //         expires: Some(cw20::Expiration::Never {  }),
    //     },
    //     &[],
    // ).unwrap();
    // suite.pass_blocks(10);

    // let allowances: AllAllowancesResponse = suite.app.wrap().query_wasm_smart(
    //     Addr::unchecked("contract6".to_string()),
    //     &cw20::Cw20QueryMsg::AllAllowances { 
    //         owner: suite.stable_pair.1.to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // ).unwrap();

    // println!("allowances response: {:?}\n", allowances);

    // suite.pass_blocks(10);
    // let tick_resp = suite.tick();
    // suite.pass_blocks(10);
    // let tick_resp = suite.tick();
    // suite.pass_blocks(10);

    let liquid_pooler_balances = suite.query_addr_balances(Addr::unchecked(suite.liquid_pooler.1.to_string()));
    println!("\n liquid pooler balances: {:?}\n", liquid_pooler_balances);

    let stable_pair_balances = suite.query_addr_balances(Addr::unchecked(suite.stable_pair.1.to_string()));
    println!("\n stable_pair_balances: {:?}\n", stable_pair_balances);

    let share_query_resp = suite.query_pool_share();
    println!("\n1 LP token can withdraw: {:?}\n", share_query_resp);

    let res: PoolResponse = suite.query_pool_info();
    println!("\nQueryMsg::Pool: {:?}\n", res);
    
    let cw20_bal = suite.query_cw20_bal(
        "contract6".to_string(),
        suite.stable_pair.1.to_string(),
    );
    println!("\ncw20_bal {:?}", cw20_bal);

    let token_info: cw20::TokenInfoResponse = suite.app.wrap().query_wasm_smart(
        Addr::unchecked("contract6".to_string()),
        &cw20::Cw20QueryMsg::TokenInfo {  },
    ).unwrap();
    println!("\ntoken info {:?}", token_info);
 


    let simulation: SimulationResponse = suite.query_simulation(suite.stable_pair.1.to_string());
    println!("\n simulation response: {:?}\n", simulation);

    // let msg = astroport::pair::ExecuteMsg:: {
    //     contract: suite.stable_pair.1.to_string(),
    //     amount: Uint128::new(10),
    //     msg: to_binary(&Cw20HookMsg::WithdrawLiquidity { assets: vec![
    //         Asset { 
    //             info: AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() },
    //             amount: Uint128::new(100),
    //         },
    //         Asset { 
    //             info: AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() },
    //             amount: Uint128::new(100),
    //         },
    //     ] }).unwrap(),
    // };

    // let liquidity_withdrawal_resp = suite.app.execute_contract(
    //     Addr::unchecked(suite.stable_pair.1.to_string()),
    //     Addr::unchecked(suite.lp_token.to_string()),
    //     &msg,
    //     &[],
    // ).unwrap();
}

#[test]
fn test_enter_lp() {
    
}