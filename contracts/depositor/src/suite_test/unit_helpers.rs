use std::marker::PhantomData;

use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    to_binary, Addr, ContractResult, Deps, DepsMut, MemoryStorage, MessageInfo, OwnedDeps,
    Response, SystemResult, Uint128, WasmQuery,
};
use neutron_sdk::{
    bindings::{
        msg::{IbcFee, NeutronMsg},
        query::NeutronQuery,
    },
    NeutronError,
};

use crate::{
    contract::{execute, instantiate, DEFAULT_TIMEOUT_SECONDS},
    msg::{ExecuteMsg, InstantiateMsg, PresetDepositorFields, WeightedReceiverAmount},
    state::{ContractState, CONTRACT_STATE},
};

pub type Owned = OwnedDeps<MemoryStorage, MockApi, MockQuerier, NeutronQuery>;

const CREATOR_ADDR: &str = "creator";
const CLOCK_ADDR: &str = "contract_clock";
const LP_ADDR: &str = "contract_lp";
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
            LS_ADDR => match from_binary::<covenant_ls::msg::QueryMsg>(&msg).unwrap() {
                covenant_ls::msg::QueryMsg::StrideICA {} => SystemResult::Ok(ContractResult::Ok(
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
        querier: MockQuerier::new(&vec![]),
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
      st_atom_receiver_amount: WeightedReceiverAmount { amount: 1 },
      atom_receiver_amount: WeightedReceiverAmount { amount: 1 },
      autopilot_format: "{{\"autopilot\": {{\"receiver\": \"{st_ica}\",\"stakeibc\": {{\"stride_address\": \"{st_ica}\",\"action\": \"LiquidStake\"}}}}}}".to_string(),
    }.to_instantiate_msg("reciever".to_string(), CLOCK_ADDR.to_string(), LS_ADDR.to_string(), LP_ADDR.to_string(), DEFAULT_TIMEOUT_SECONDS, get_default_ibc_fee())
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
