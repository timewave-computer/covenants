
use cosmwasm_std::{from_binary, Addr, coins, BankMsg, CosmosMsg, Timestamp};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::contract::{instantiate, query, execute};
use crate::error::ContractError;


// Queried withdrawer address is same as one we instantiated with
#[test]
fn test_instantiate_query_withdrawer() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let test_addr = "anaddressonneutron";
    let instantiate_msg = InstantiateMsg {
        withdrawer: Some(test_addr.to_string())
    };

    instantiate(
        deps.as_mut(),
        env.clone(),
        mock_info("sender", &[]),
        instantiate_msg
    )
    .unwrap();

    let resp = query(deps.as_ref(), env, QueryMsg::Withdrawer {}).unwrap();
    let resp: Addr = from_binary(&resp).unwrap();

    // assert that instantiated address = queried address
    assert_eq!(resp, Addr::unchecked("anaddressonneutron"));

    // and for posterity check that a different address fails
    assert_ne!(resp, Addr::unchecked("anotheraddress"));
}
// Test withdraw
// 1. A sender who is not the withdrawer is unauthorized
// 2. Authorized withdrawer can withdraw partial amount
// 3. Authorized withdrawer can withdraw complete balance

#[test]
fn test_withdraw() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let contract_addr = env.clone().contract.address;
    let test_addr = "authorized";
    let instantiate_msg = InstantiateMsg {
        withdrawer: Some(test_addr.to_string())
    };

    // Initally set up the balance with 1000 coin
    let init_amount = coins(1000, "coin");

    instantiate(
        deps.as_mut(),
        env.clone(),
        mock_info("sender", &init_amount),
        instantiate_msg
    )
    .unwrap();
    
    deps.querier.update_balance(&contract_addr, init_amount);

    // We will first try to withdraw 250 coin
    let execute_msg = ExecuteMsg::Withdraw {
        quantity: Some(coins(250, "coin"))
    };
    
    // If an unauthorized address tries to withdraw
    let unauthorized_info = mock_info("unauthorized", &[]);
    let resp = execute(deps.as_mut(), env, unauthorized_info, execute_msg.clone());

    // We expect to see an unauthorized error
    match resp.unwrap_err() {
        ContractError::Unauthorized { .. } => {}
        e => panic!("unexpected error: {:?}", e),
    }

    // A little while later an authorized address tries to partially withdraw
    let mut env = mock_env();
    env.block.height = 10;
    env.block.time = Timestamp::from_seconds(0);

    let authorized_info = mock_info("authorized", &[]);
    let resp = execute(deps.as_mut(), env, authorized_info.clone(), execute_msg.clone()).unwrap();
    
    // we should expect to see 1 bank message with partial send
    assert_eq!(1, resp.messages.len());
    let msg = resp.messages.get(0).expect("no message");
    assert_eq!(
        msg.msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "authorized".into(),
            amount: coins(250, "coin"),
        })
    );

    let remaining_balance = coins(750, "coin");
    deps.querier.update_balance(&contract_addr, remaining_balance);

    // A little while later try withdrawing everything
    let mut env = mock_env();
    env.block.height = 20;
    env.block.time = Timestamp::from_seconds(0);

    // Specify no quantity here
    let execute_msg = ExecuteMsg::Withdraw {
        quantity: None
    };
    let resp = execute(deps.as_mut(), env, authorized_info.clone(), execute_msg.clone()).unwrap();
    // we should expect to see 1 bank message with partial send
    assert_eq!(1, resp.messages.len());
    let msg = resp.messages.get(0).expect("no message");
    assert_eq!(
        msg.msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "authorized".into(),
            amount: coins(750, "coin"),
        })
    );
}