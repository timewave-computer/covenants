use cosmwasm_std::{testing::mock_env, to_binary, Binary, CosmosMsg, WasmMsg};
use neutron_sdk::bindings::{msg::NeutronMsg, types::ProtobufAny};

use crate::{
    contract::{sudo, DEFAULT_TIMEOUT_SECONDS, INTERCHAIN_ACCOUNT_ID},
    state::ContractState,
    suite_test::unit_helpers::{
        get_default_ibc_fee, get_default_init_msg, get_default_msg_transfer,
        get_default_sudo_open_ack, to_proto, CLOCK_ADDR, LP_ADDR, execute_received,
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
    let default_init_msg = get_default_init_msg();

    let stride_transfer_msg = get_default_msg_transfer();

    let mut lp_transfer_msg = get_default_msg_transfer();
    lp_transfer_msg.source_channel = default_init_msg.gaia_neutron_ibc_transfer_channel_id;
    lp_transfer_msg.receiver = LP_ADDR.to_string();

    verify_state(&deps, ContractState::FundsSent);
    assert_eq!(tick_res.messages.len(), 1);
    assert_eq!(
        tick_res.messages[0].msg,
        CosmosMsg::Custom(NeutronMsg::SubmitTx {
            connection_id: default_version.controller_connection_id,
            interchain_account_id: INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs: vec![
                ProtobufAny {
                    type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                    value: Binary::from(to_proto(stride_transfer_msg)),
                },
                ProtobufAny {
                    type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                    value: Binary::from(to_proto(lp_transfer_msg)),
                }
            ],
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

    // We are waiting for the received msg from lper to move state,
    // So the tick should be successful and state stay as FundsSent
    do_tick(deps.as_mut()).unwrap();
    verify_state(&deps, ContractState::FundsSent);
}

#[test]
fn test_received() {
    let (mut deps, _) = do_instantiate();
    let default_init_msg = get_default_init_msg();

    // tick 1
    deps = do_tick_1(deps);
    //tick 2
    do_tick(deps.as_mut()).unwrap();

    // MUST error, because sent not by the lper
    execute_received(deps.as_mut(), "random_addr").unwrap_err();

    //Do recieved from the lper
    let res = execute_received(deps.as_mut(), default_init_msg.atom_receiver.address.as_str()).unwrap();

    verify_state(&deps, ContractState::Complete);
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
      res.messages[0].msg,
        WasmMsg::Execute {
            contract_addr: CLOCK_ADDR.to_string(),
            msg: to_binary(&covenant_clock::msg::ExecuteMsg::Dequeue {}).unwrap(),
            funds: vec![]
        }
        .into()
    );
    println!("{:?}", res);
}
