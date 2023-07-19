use cosmwasm_std::{testing::mock_env, CosmosMsg, WasmMsg, DepsMut};
use neutron_sdk::{bindings::{msg::NeutronMsg, query::NeutronQuery}, sudo::msg::SudoMsg};

use crate::{
    contract::{sudo, INTERCHAIN_ACCOUNT_ID},
    msg::OpenAckVersion,
    state::ContractState,
    suite_test::unit_helpers::get_default_init_msg,
};

use super::unit_helpers::{do_instantiate, do_tick, verify_state, Owned};

#[test]
fn test_init() {
    let (deps, _) = do_instantiate();
    verify_state(&deps, ContractState::Instantiated)
}

fn do_tick_1(mut deps: Owned) -> Owned{
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
    let counterparty_version = OpenAckVersion {
        version: "ica".to_string(),
        controller_connection_id: "connection-0".to_string(),
        host_connection_id: "connection-1".to_string(),
        address: "ica_addr".to_string(),
        encoding: "json".to_string(),
        tx_type: "register".to_string(),
    };
    let counterparty_version = serde_json_wasm::to_string(&counterparty_version).unwrap();
    let sudo_msg = SudoMsg::OpenAck {
        port_id: "port-1".to_string(),
        channel_id: "channel-0".to_string(),
        counterparty_channel_id: "channel-1".to_string(),
        counterparty_version,
    };

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
  println!("{:?}", tick_res);
}
