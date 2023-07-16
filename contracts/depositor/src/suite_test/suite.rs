use cosmwasm_std::{testing::MockApi, Addr, Empty, MemoryStorage, Uint128};
use cw_multi_test::{
    App, BankKeeper, BasicAppBuilder, Contract, ContractWrapper, Executor, FailingModule,
    WasmKeeper,
};
use neutron_sdk::bindings::{msg::{NeutronMsg, IbcFee}, query::NeutronQuery};

use crate::msg::{InstantiateMsg, QueryMsg, WeightedReceiver};

pub const CREATOR_ADDR: &str = "creator";
pub const NEUTRON_DENOM: &str = "untrn";
pub const ST_ATOM_DENOM: &str = "stride-atom";
pub const NATIVE_ATOM_DENOM: &str = "native-atom";
pub const _DEFAULT_RECEIVER_AMOUNT: Uint128 = Uint128::new(10);
pub const _DEFAULT_CLOCK_ADDRESS: &str = "clock-address";

// pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, MockQuerier, NeutronQuery> {
//     OwnedDeps {
//         storage: MockStorage::default(),
//         api: MockApi::default(),
//         querier: MockQuerier::default(),
//         custom_query_type: PhantomData,
//     }
// }

// fn depositor_contract() -> Box<dyn Contract<NeutronResult<NeutronError>>> {
//     // let contract = ContractWrapper::new(
//     //     crate::contract::execute,
//     //     crate::contract::instantiate,
//     //     query,
//     // );
//     // // todo
//     // Box::new(contract)
//     let execute_func =
//     Box::new(ContractWrapper::new(execute_fn, instantiate_fn, query_fn))
// }

#[allow(unused)]
pub(crate) struct Suite {
    pub app: BaseApp,
    pub admin: Addr,
    pub depositor_address: Addr,
    pub depositor_code: u64,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                st_atom_receiver: WeightedReceiver {
                    amount: 10,
                    address: ST_ATOM_DENOM.to_string(),
                },
                atom_receiver: WeightedReceiver {
                    amount: 10,
                    address: NATIVE_ATOM_DENOM.to_string(),
                },
                clock_address: "default-clock".to_string(),
                gaia_neutron_ibc_transfer_channel_id: "channel-3".to_string(),
                neutron_gaia_connection_id: "connection-0".to_string(),
                gaia_stride_ibc_transfer_channel_id: "channel-3".to_string(),
                ls_address: "TODO".to_string(),
                ibc_timeout: 100000,
                ibc_fee: IbcFee {
                    recv_fee: vec![], // must be empty
                    ack_fee: vec![cosmwasm_std::Coin {
                        denom: NEUTRON_DENOM.to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                    timeout_fee: vec![cosmwasm_std::Coin {
                        denom: NEUTRON_DENOM.to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                },
            },
        }
    }
}

fn depositor_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    // todo
    Box::new(contract)
}

pub type BaseApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<NeutronMsg, NeutronQuery, Empty>,
    WasmKeeper<NeutronMsg, NeutronQuery>,
>;

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = BasicAppBuilder::<NeutronMsg, NeutronQuery>::new_custom().build(|_, _, _| {});
        // app.store_code()
        let depositor_code = app.store_code(depositor_contract());

        let depositor_address = app
            .instantiate_contract(
                depositor_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.instantiate,
                &[],
                "depositor contract",
                Some(CREATOR_ADDR.to_string()),
            )
            .unwrap();

        Suite {
            app,
            admin: Addr::unchecked(CREATOR_ADDR),
            depositor_address,
            depositor_code,
        }
    }
}

// queries
impl Suite {
    pub fn query_stride_atom_receiver(&self) -> WeightedReceiver {
        self.app
            .wrap()
            .query_wasm_smart(&self.depositor_address, &QueryMsg::StAtomReceiver {})
            .unwrap()
    }

    pub fn query_native_atom_receiver(&self) -> WeightedReceiver {
        self.app
            .wrap()
            .query_wasm_smart(&self.depositor_address, &QueryMsg::AtomReceiver {})
            .unwrap()
    }

    pub fn query_clock_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.depositor_address, &QueryMsg::ClockAddress {})
            .unwrap()
    }
}

// assertion helpers
impl Suite {
    #[allow(unused)]
    pub fn assert_stride_atom_receiver(&self, val: WeightedReceiver) {
        let curr = self.query_stride_atom_receiver();
        assert_eq!(curr, val);
    }

    #[allow(unused)]
    pub fn assert_native_atom_receiver(&self, val: WeightedReceiver) {
        let curr = self.query_native_atom_receiver();
        assert_eq!(curr, val);
    }
    #[allow(unused)]
    pub fn assert_clock_address(&self, val: Addr) {
        let curr = self.query_clock_address();
        assert_eq!(curr, val);
    }
}
