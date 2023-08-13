use std::marker::PhantomData;

use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, ibc::applications::transfer::v1::MsgTransfer};
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, ContractResult, DepsMut, MemoryStorage, MessageInfo, OwnedDeps, Response,
    SystemResult, Uint128, Uint64, WasmQuery,
};
use neutron_sdk::{
    bindings::{
        msg::{IbcFee, NeutronMsg},
        query::NeutronQuery,
    },
    interchain_txs::helpers::get_port_id,
    sudo::msg::SudoMsg,
    NeutronError,
};
use prost::Message;

use crate::{
    contract::{execute, instantiate, INTERCHAIN_ACCOUNT_ID},
    msg::{
        ExecuteMsg, InstantiateMsg, OpenAckVersion, PresetDepositorFields, WeightedReceiverAmount, ContractState,
    },
    state::CONTRACT_STATE,
};

pub const CREATOR_ADDR: &str = "creator";
pub const _NEUTRON_DENOM: &str = "untrn";
pub const _ST_ATOM_DENOM: &str = "statom";
pub const NATIVE_ATOM_DENOM: &str = "uatom";

pub type Owned = OwnedDeps<MemoryStorage, MockApi, MockQuerier, NeutronQuery>;

pub const CLOCK_ADDR: &str = "contract_clock";
pub const LP_ADDR: &str = "contract_lp";
const LS_ADDR: &str = "contract_ls";

pub(crate) fn get_default_ibc_fee() -> IbcFee {
    IbcFee {
        recv_fee: vec![],
        ack_fee: vec![cosmwasm_std::Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000u128),
        }],
        timeout_fee: vec![cosmwasm_std::Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000u128),
        }],
    }
}

pub fn wasm_handler(wasm_query: &WasmQuery) -> SystemResult<ContractResult<cosmwasm_std::Binary>> {
    match wasm_query {
        WasmQuery::Smart { contract_addr, msg } => match contract_addr.as_ref() {
            LS_ADDR => match from_binary::<covenant_ls::msg::QueryMsg>(msg).unwrap() {
                covenant_ls::msg::QueryMsg::IcaAddress {} => SystemResult::Ok(ContractResult::Ok(
                    to_binary(&Addr::unchecked("some_ica_addr")).unwrap(),
                )),
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, MockQuerier, NeutronQuery> {
    let mut owned_deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::new(&[]),
        custom_query_type: PhantomData,
    };

    owned_deps.querier.update_wasm(wasm_handler);
    owned_deps
}

pub(crate) fn get_default_init_msg() -> InstantiateMsg {
    PresetDepositorFields {
      gaia_neutron_ibc_transfer_channel_id: "channel-0".to_string(),
      neutron_gaia_connection_id: "connection-0".to_string(),
      gaia_stride_ibc_transfer_channel_id: "channel-1".to_string(),
      depositor_code: 1,
      label: "depositor".to_string(),
      st_atom_receiver_amount: WeightedReceiverAmount { amount: Uint128::new(1000) },
      atom_receiver_amount: WeightedReceiverAmount { amount: Uint128::new(1000) },
      autopilot_format: "{\"autopilot\": {\"receiver\": \"{st_ica}\",\"stakeibc\": {\"stride_address\": \"{st_ica}\",\"action\": \"LiquidStake\"}}}".to_string(),
      neutron_atom_ibc_denom: "uatom".to_string(),
    }.to_instantiate_msg(
        "reciever".to_string(),
        CLOCK_ADDR.to_string(),
        LS_ADDR.to_string(),
        LP_ADDR.to_string(),
        get_default_ibc_fee(),
        Uint64::new(100),
        Uint64::new(100),
    )
}

pub fn get_default_sudo_open_ack() -> (SudoMsg, OpenAckVersion) {
    let counterparty_version = OpenAckVersion {
        version: "ica".to_string(),
        controller_connection_id: "connection-0".to_string(),
        host_connection_id: "connection-1".to_string(),
        address: "ica_addr".to_string(),
        encoding: "json".to_string(),
        tx_type: "register".to_string(),
    };
    let json_counterparty_version = serde_json_wasm::to_string(&counterparty_version).unwrap();
    let sudo_msg = SudoMsg::OpenAck {
        port_id: get_port_id(MOCK_CONTRACT_ADDR, INTERCHAIN_ACCOUNT_ID),
        channel_id: "channel-0".to_string(),
        counterparty_channel_id: "channel-1".to_string(),
        counterparty_version: json_counterparty_version,
    };

    (sudo_msg, counterparty_version)
}

pub fn get_default_msg_transfer() -> MsgTransfer {
    let default_init_msg = get_default_init_msg();
    let (_, default_sudo_open_ack) = get_default_sudo_open_ack();

    MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: default_init_msg.gaia_stride_ibc_transfer_channel_id,
        token: Some(Coin {
            denom: NATIVE_ATOM_DENOM.to_string(),
            amount: default_init_msg.atom_receiver.amount.to_string(),
        }),
        sender: default_sudo_open_ack.address,
        receiver: default_init_msg
            .autopilot_format
            .replace("{st_ica}", "some_ica_addr"),
        timeout_height: None,
        timeout_timestamp: 99999999999,
    }
}

pub fn to_proto(to_proto: impl Message) -> Vec<u8> {
    // Serialize the Transfer message
    let mut buf = Vec::new();
    buf.reserve(to_proto.encoded_len());
    to_proto.encode(&mut buf).unwrap();
    buf
}

pub fn do_instantiate() -> (Owned, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR_ADDR, &[]);
    let init_msg = get_default_init_msg();

    instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
    (deps, info)
}

pub fn do_tick(deps: DepsMut<NeutronQuery>) -> Result<Response<NeutronMsg>, NeutronError> {
    let info = mock_info(CLOCK_ADDR, &[]);
    execute(deps, mock_env(), info, ExecuteMsg::Tick {})
}

pub fn verify_state(deps: &Owned, contract_state: ContractState) {
    let state = CONTRACT_STATE.load(deps.as_ref().storage).unwrap();
    assert_eq!(state, contract_state)
}
