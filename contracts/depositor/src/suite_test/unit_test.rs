use cosmwasm_std::{
    testing::mock_env,
    Binary, CosmosMsg,  WasmMsg, to_binary,
};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, types::ProtobufAny},
};

use crate::{
    contract::{sudo, DEFAULT_TIMEOUT_SECONDS, INTERCHAIN_ACCOUNT_ID},
    state::{ContractState},
    suite_test::unit_helpers::{
        get_default_ibc_fee, get_default_init_msg, get_default_msg_transfer,
        get_default_sudo_open_ack, to_proto, CLOCK_ADDR, LP_ADDR,
    },
};

use super::unit_helpers::{do_instantiate, do_tick, verify_state, Owned};

#[test]
fn test_init() {
    let (deps, _) = do_instantiate();
    verify_state(&deps, ContractState::Instantiated)
}

fn do_tick_1(mut deps: Owned) -> Owned {
    let tick_res = do_tick(deps.as_mut()).unwrap();

    let default_init_msg = get_default_init_msg();

    verify_state(&deps, ContractState::Instantiated);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::RegisterInterchainAccount {
            connection_id: default_init_msg.neutron_gaia_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
        })
    );

    //sudo response from neutron
    let (sudo_msg, _) = get_default_sudo_open_ack();

    sudo(deps.as_mut(), mock_env(), sudo_msg).unwrap();
    deps
}

#[test]
fn test_tick_1() {
    let (mut deps, _) = do_instantiate();

    deps = do_tick_1(deps);
    verify_state(&deps, ContractState::ICACreated);
}

#[test]
fn test_tick_2() {
    let (mut deps, _) = do_instantiate();

    deps = do_tick_1(deps);

    let tick_res = do_tick(deps.as_mut()).unwrap();
    let (_, default_version) = get_default_sudo_open_ack();
    let msg_transfer = get_default_msg_transfer();

    verify_state(&deps, ContractState::LiquidStaked);
    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::SubmitTx {
            connection_id: default_version.controller_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs: vec![ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(to_proto(msg_transfer)),
            }],
            memo: "".to_string(),
            timeout: DEFAULT_TIMEOUT_SECONDS,
            fee: get_default_ibc_fee()
        })
    );
}

#[test]
fn test_tick_3() {
    let (mut deps, _) = do_instantiate();

    // tick 1
    deps = do_tick_1(deps);
    //tick 2
    do_tick(deps.as_mut()).unwrap();
    //tick 3
    let tick_res = do_tick(deps.as_mut()).unwrap();

    let default_init_msg: crate::msg::InstantiateMsg = get_default_init_msg();
    let (_, default_version) = get_default_sudo_open_ack();
    let mut msg_transfer = get_default_msg_transfer();
    msg_transfer.source_channel = default_init_msg.gaia_neutron_ibc_transfer_channel_id;
    msg_transfer.receiver = LP_ADDR.to_string();

    verify_state(&deps, ContractState::Complete);
    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::SubmitTx {
            connection_id: default_version.controller_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs: vec![ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(to_proto(msg_transfer)),
            }],
            memo: "ica_addr".to_string(),
            timeout: DEFAULT_TIMEOUT_SECONDS,
            fee: get_default_ibc_fee()
        })
    );
}

#[test]
fn test_tick_4() {
    let (mut deps, _) = do_instantiate();

    // tick 1
    deps = do_tick_1(deps);
    //tick 2
    do_tick(deps.as_mut()).unwrap();
    //tick 3
    do_tick(deps.as_mut()).unwrap();
    //tick 4
    let tick_res = do_tick(deps.as_mut()).unwrap();

    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        WasmMsg::Execute {
            contract_addr: CLOCK_ADDR.to_string(),
            msg: to_binary(&covenant_clock::msg::ExecuteMsg::Dequeue {}).unwrap(),
            funds: vec![]
        }.into()
    );
}
