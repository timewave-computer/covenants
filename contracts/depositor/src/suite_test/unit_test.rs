use cosmwasm_std::{coins, testing::{mock_env, MockApi, MockQuerier}, to_binary, Binary, CosmosMsg, WasmMsg, Reply, SubMsgResponse, from_binary, OwnedDeps, MemoryStorage};
use neutron_sdk::{bindings::{msg::{NeutronMsg, MsgSubmitTxResponse}, types::ProtobufAny, query::NeutronQuery}, sudo::msg::{SudoMsg, RequestPacket}};

use crate::{
    contract::{sudo, INTERCHAIN_ACCOUNT_ID, SUDO_PAYLOAD_REPLY_ID, to_proto_msg_transfer},
    msg::ContractState,
    suite_test::unit_helpers::{
        get_default_ibc_fee, get_default_init_msg, get_default_msg_transfer,
        get_default_sudo_open_ack, to_proto, CLOCK_ADDR, LP_ADDR, NATIVE_ATOM_DENOM, sudo_execute, reply_execute,
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

// This test should send the native token to the lper and set state to VerifyNativeToken
#[test]
fn test_tick_2() {
    let (mut deps, _) = do_instantiate();

    deps = do_tick_1(deps);

    let tick_res = do_tick(deps.as_mut()).unwrap();
    let (_, default_version) = get_default_sudo_open_ack();
    let default_init_msg = get_default_init_msg();

    let mut lp_transfer_msg = get_default_msg_transfer();
    lp_transfer_msg.source_channel = default_init_msg.gaia_neutron_ibc_transfer_channel_id;
    lp_transfer_msg.receiver = LP_ADDR.to_string();
    // env.block.time + ibc transfer timeout (100sec)
    lp_transfer_msg.timeout_timestamp = 1571797619879305533;
    reply_execute(deps.as_mut(), Reply {
        id: SUDO_PAYLOAD_REPLY_ID,
        result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(to_binary(&MsgSubmitTxResponse {
                sequence_id: 1,
                channel: "channel-0".to_string(),
            }).unwrap()),
        }),
    }).unwrap();
    sudo_execute(deps.as_mut(), SudoMsg::Response {
        request: RequestPacket {
            sequence: Some(1),
            source_port: None,
            source_channel: Some("channel-0".to_string()),
            destination_port: None,
            destination_channel: None,
            data: None,
            timeout_height: None,
            timeout_timestamp: None,
        },
        data: to_binary(&1).unwrap(),
    }).unwrap();
    verify_state(&deps, ContractState::VerifyNativeToken);
    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::SubmitTx {
            connection_id: default_version.controller_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs: vec![ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(to_proto(lp_transfer_msg)),
            }],
            memo: "".to_string(),
            timeout: 100,
            fee: get_default_ibc_fee()
        })
    );
}

// This tick should verify lper got native token, and send st tokens to the lper
#[test]
fn test_tick_3() {
    let (mut deps, _) = do_instantiate();
    
    // tick 1
    deps = do_tick_1(deps);
    //tick 2
    do_tick(deps.as_mut()).unwrap();
    reply_execute(deps.as_mut(), Reply {
        id: SUDO_PAYLOAD_REPLY_ID,
        result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(to_binary(&MsgSubmitTxResponse {
                sequence_id: 1,
                channel: "channel-0".to_string(),
            }).unwrap()),
        }),
    }).unwrap();
    sudo_execute(deps.as_mut(), SudoMsg::Response {
        request: RequestPacket {
            sequence: Some(1),
            source_port: None,
            source_channel: Some("channel-0".to_string()),
            destination_port: None,
            destination_channel: None,
            data: None,
            timeout_height: None,
            timeout_timestamp: None,
        },
        data: to_binary(&1).unwrap(),
    }).unwrap();
    verify_state(&deps, ContractState::VerifyNativeToken);


    // Increase balance of lper
    deps.querier
        .update_balance(LP_ADDR, coins(1000, NATIVE_ATOM_DENOM));
    // Balance is incorrect, so it should still be in VerifyNativeToken state
    do_tick(deps.as_mut()).unwrap();

    // do another tick
    let tick_res = do_tick(deps.as_mut()).unwrap();

    let mut stride_transfer_msg = get_default_msg_transfer();
    stride_transfer_msg.timeout_timestamp = 1571797619879305533;

    let proto_msg = to_proto_msg_transfer(stride_transfer_msg).unwrap();
    let (_, default_version) = get_default_sudo_open_ack();

    let msg = CosmosMsg::Custom(NeutronMsg::SubmitTx {
        connection_id: default_version.controller_connection_id,
        interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
        msgs: vec![proto_msg],
        memo: "".to_string(),
        timeout: 100,
        fee: get_default_ibc_fee()
    });
    verify_state(&deps, ContractState::VerifyLp);
    assert_eq!(
        tick_res.messages[0].msg,
        msg,
    );
}

// This tests the final tick, where the balance of the lper is reduced to 0
#[test]
fn test_tick_4() {
    let (mut deps, _) = do_instantiate();

    // tick 1
    deps = do_tick_1(deps);
 
    //tick 2
    do_tick(deps.as_mut()).unwrap();
    reply_execute(deps.as_mut(), Reply {
        id: SUDO_PAYLOAD_REPLY_ID,
        result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(to_binary(&MsgSubmitTxResponse {
                sequence_id: 1,
                channel: "channel-0".to_string(),
            }).unwrap()),
        }),
    }).unwrap();
    sudo_execute(deps.as_mut(), SudoMsg::Response {
        request: RequestPacket {
            sequence: Some(1),
            source_port: None,
            source_channel: Some("channel-0".to_string()),
            destination_port: None,
            destination_channel: None,
            data: None,
            timeout_height: None,
            timeout_timestamp: None,
        },
        data: to_binary(&1).unwrap(),
    }).unwrap();
    // Increase balance of lper
    deps.querier
        .update_balance(LP_ADDR, coins(1000, NATIVE_ATOM_DENOM));
    // tick 3
    do_tick(deps.as_mut()).unwrap();

    // balance wasnt reduced yet, so we should try transfer to stride again
    let tick_res = do_tick(deps.as_mut()).unwrap();

    let mut stride_transfer_msg = get_default_msg_transfer();
    stride_transfer_msg.timeout_timestamp = 1571797619879305533;
    let (_, default_version) = get_default_sudo_open_ack();

    verify_state(&deps, ContractState::VerifyLp);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::SubmitTx {
            connection_id: default_version.controller_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs: vec![ProtobufAny {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: Binary::from(to_proto(stride_transfer_msg)),
            }],
            memo: "".to_string(),
            timeout: 100,
            fee: get_default_ibc_fee()
        })
    );

    // reduce balance to 0, should change state to complete and dequeue from clock
    deps.querier
        .update_balance(LP_ADDR, coins(0, NATIVE_ATOM_DENOM));

    let tick_res = do_tick(deps.as_mut()).unwrap();

    verify_state(&deps, ContractState::Complete);
    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        WasmMsg::Execute {
            contract_addr: CLOCK_ADDR.to_string(),
            msg: to_binary(&covenant_clock::msg::ExecuteMsg::Dequeue {}).unwrap(),
            funds: vec![]
        }
        .into()
    );
}
