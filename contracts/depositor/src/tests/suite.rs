use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{InstantiateMsg, QueryMsg, WeightedReceiver};

pub const CREATOR_ADDR: &str = "creator";
pub const ST_ATOM_DENOM: &str = "stride-atom";
pub const NATIVE_ATOM_DENOM: &str = "native-atom";
pub const DEFAULT_RECEIVER_AMOUNT: Uint128 = Uint128::new(10);
pub const DEFAULT_CLOCK_ADDRESS: &str = "clock-address";

// fn depositor_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         crate::contract::execute,
//         crate::contract::instantiate,
//         crate::contract::query,
//     );
//     // todo
//     Box::new(contract)
// }

pub(crate) struct Suite {
    app: App,
    pub _admin: Addr,
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
            },
        }
    }
}

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = App::default();

        // let depositor_code = app.store_code(depositor_contract());
        let depositor_code = 1;
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
            _admin: Addr::unchecked(CREATOR_ADDR),
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
    pub fn assert_stride_atom_receiver(&self, val: WeightedReceiver) {
        let curr = self.query_stride_atom_receiver();
        assert_eq!(curr, val);
    }

    pub fn assert_native_atom_receiver(&self, val: WeightedReceiver) {
        let curr = self.query_native_atom_receiver();
        assert_eq!(curr, val);
    }

    pub fn assert_clock_address(&self, val: Addr) {
        let curr = self.query_clock_address();
        assert_eq!(curr, val);
    }
}