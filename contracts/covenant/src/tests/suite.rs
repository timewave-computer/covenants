
use cosmwasm_std::{Uint64, Empty, Addr};
use covenant_depositor::msg::WeightedReceiver;
use cw_multi_test::{App, ContractWrapper, Contract, Executor};


use crate::msg::{InstantiateMsg, QueryMsg};

pub const ST_ATOM_DENOM: &str = "stuatom";
pub const NATIVE_ATOM_DENOM: &str = "uatom";

pub const CREATOR_ADDR: &str = "admin";
pub const TODO: &str = "replace";



fn covenant_clock() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            covenant_clock::contract::execute,
            covenant_clock::contract::instantiate,
            covenant_clock::contract::query
        ) 
        .with_reply(covenant_clock::contract::reply)
        .with_migrate(covenant_clock::contract::migrate)
    ) 
}

fn covenant_holder() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            covenant_holder::contract::execute,
            covenant_holder::contract::instantiate,
            covenant_holder::contract::query
        )
    ) 
}

fn covenant_covenant() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query
        ) 
        .with_reply(crate::contract::reply)
        .with_migrate(crate::contract::migrate)
    ) 
}

pub(crate) struct Suite {
    pub app: App,
    pub covenant_address: Addr,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                ls_instantiate: covenant_ls::msg::InstantiateMsg {
                    autopilot_position: TODO.to_string(),
                    clock_address: TODO.to_string(),
                    stride_neutron_ibc_transfer_channel_id: TODO.to_string(),
                    neutron_stride_ibc_connection_id: TODO.to_string(),
                    lp_address: TODO.to_string(),
                    ls_denom: TODO.to_string(),
                },
                depositor_instantiate: covenant_depositor::msg::InstantiateMsg {
                    st_atom_receiver: WeightedReceiver {
                        amount: 1,
                        address: TODO.to_string(),
                    },
                    atom_receiver: WeightedReceiver {
                        amount: 1,
                        address: TODO.to_string(),
                    },
                    clock_address: TODO.to_string(),
                    gaia_neutron_ibc_transfer_channel_id: TODO.to_string(),
                    neutron_gaia_connection_id: TODO.to_string(),
                    gaia_stride_ibc_transfer_channel_id: TODO.to_string(),
                    ls_address: TODO.to_string(),
                },
                lp_instantiate: covenant_lp::msg::InstantiateMsg {
                    lp_position: covenant_lp::msg::LPInfo { addr: TODO.to_string() },
                    clock_address: TODO.to_string(),
                    holder_address: TODO.to_string(),
                    assets: vec![
                        // Asset {
                        //     info: todo!(), amount: todo!()
                        // },
                        // Asset { info: todo!(), amount: todo!() }
                    ],
                    slippage_tolerance: None,
                    autostake: Some(false),
                },
                clock_code: 1,
                clock_instantiate: covenant_clock::msg::InstantiateMsg {
                    tick_max_gas: Uint64::new(10000),
                },
                ls_code: 1,
                depositor_code: 1,
                lp_code: 1,
                holder_code: 1,
                holder_instantiate: covenant_holder::msg::InstantiateMsg {
                    withdrawer: Some(CREATOR_ADDR.to_string()),
                }
            },
        }
    }
}


impl SuiteBuilder {
    pub fn build(mut self) -> Suite {
        let mut app = App::default();

        self.instantiate.holder_code = app.store_code(covenant_holder());
        self.instantiate.clock_code = app.store_code(covenant_clock());
        let covenant_code = app.store_code(covenant_covenant());

        let _ls_contract = Box::new(
            ContractWrapper::new(
                covenant_ls::contract::execute,
                covenant_ls::contract::instantiate,
                covenant_clock::contract::query,
            )
        );

        let _depositor_contract = Box::new(
            ContractWrapper::new(
                covenant_depositor::contract::execute,
                covenant_depositor::contract::instantiate,
                covenant_clock::contract::query,
            )
        );

        
        let covenant_address = app
            .instantiate_contract(
                covenant_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.instantiate,
                &[],
                "covenant contract",
                Some(CREATOR_ADDR.to_string()),
            )
            .unwrap();

        
        Suite {
            app,
            covenant_address,
        }
    }

    // pub fn with_ls(mut self, instantiate_msg: covenant_ls::msg::InstantiateMsg) -> Self {
    //     self.instantiate.ls_instantiate = instantiate_msg;
    //     self
    // }

    // pub fn with_lp(mut self, instantiate_msg: covenant_lp::msg::InstantiateMsg) -> Self {
    //     self.instantiate.lp_instantiate = instantiate_msg;
    //     self
    // }

    // pub fn with_depositor(mut self, instantiate_msg: covenant_depositor::msg::InstantiateMsg) -> Self {
    //     self.instantiate.depositor_instantiate = instantiate_msg;
    //     self
    // }
}

// assertion helpers
impl Suite {}

// queries
impl Suite {
    pub fn query_clock_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.covenant_address,
                &QueryMsg::ClockAddress {}
            )    
            .unwrap()
    }

    pub fn query_holder_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.covenant_address,
                &QueryMsg::HolderAddress {}
            )    
            .unwrap()
    }

    pub fn query_lp_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.covenant_address,
                &QueryMsg::LpAddress {}
            )    
            .unwrap()
    }

    pub fn query_ls_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.covenant_address,
                &QueryMsg::LsAddress {}
            )    
            .unwrap()
    }

    pub fn query_depositor_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.covenant_address,
                &QueryMsg::DepositorAddress {}
            )    
            .unwrap()
    }
}