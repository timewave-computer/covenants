use cosmwasm_std::{Addr, Empty, Uint64};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{InstantiateMsg, QueryMsg};

pub const CREATOR_ADDR: &str = "admin";
pub const TODO: &str = "replace";

fn covenant_clock() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            covenant_clock::contract::execute,
            covenant_clock::contract::instantiate,
            covenant_clock::contract::query,
        )
        .with_reply(covenant_clock::contract::reply)
        .with_migrate(covenant_clock::contract::migrate),
    )
}

fn covenant_holder() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        covenant_holder::contract::execute,
        covenant_holder::contract::instantiate,
        covenant_holder::contract::query,
    ))
}

fn covenant_covenant() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply)
        .with_migrate(crate::contract::migrate),
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
                preset_clock_fields: covenant_clock::msg::PresetClockFields {
                    tick_max_gas: Some(Uint64::new(10000)),
                    clock_code: 1,
                    label: "covenant_clock_contract".to_string(),
                },
                preset_ls_fields: covenant_ls::msg::PresetLsFields {
                    ls_code: 1,
                    label: "covenant_ls_contract".to_string(),
                    ls_denom: "stuatom".to_string(),
                    stride_neutron_ibc_transfer_channel_id: TODO.to_string(),
                    neutron_stride_ibc_connection_id: TODO.to_string(),
                    lp_address: TODO.to_string(),
                },
                preset_depositor_fields: covenant_depositor::msg::PresetDepositorFields {
                    gaia_neutron_ibc_transfer_channel_id: TODO.to_string(),
                    neutron_gaia_connection_id: TODO.to_string(),
                    gaia_stride_ibc_transfer_channel_id: TODO.to_string(),
                    depositor_code: 1,
                    label: "covenant_depositor_contract".to_string(),
                    st_atom_receiver_amount: covenant_depositor::msg::WeightedReceiverAmount {
                        amount: 1,
                    },
                    atom_receiver_amount: covenant_depositor::msg::WeightedReceiverAmount {
                        amount: 1,
                    },
                },
                preset_lp_fields: covenant_lp::msg::PresetLpFields {
                    slippage_tolerance: None,
                    autostake: Some(false),
                    assets: vec![],
                    lp_code: 1,
                    lp_position: TODO.to_string(),
                    label: "covenant_lp_contract".to_string(),
                    single_side_lp_limit: None,
                },
                preset_holder_fields: covenant_holder::msg::PresetHolderFields {
                    withdrawer: Some(CREATOR_ADDR.to_string()),
                    holder_code: 1,
                    label: "covenant_holder_contract".to_string(),
                },
                label: "covenant_contract".to_string(),
            },
        }
    }
}

impl SuiteBuilder {
    pub fn build(mut self) -> Suite {
        let mut app = App::default();

        self.instantiate.preset_holder_fields.holder_code = app.store_code(covenant_holder());
        self.instantiate.preset_clock_fields.clock_code = app.store_code(covenant_clock());
        let covenant_code = app.store_code(covenant_covenant());

        let _ls_contract = Box::new(ContractWrapper::new(
            covenant_ls::contract::execute,
            covenant_ls::contract::instantiate,
            covenant_clock::contract::query,
        ));

        let _depositor_contract = Box::new(ContractWrapper::new(
            covenant_depositor::contract::execute,
            covenant_depositor::contract::instantiate,
            covenant_depositor::contract::query,
        ));

        let covenant_address = app
            .instantiate_contract(
                covenant_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.instantiate.clone(),
                &[],
                self.instantiate.label,
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
            .query_wasm_smart(&self.covenant_address, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_holder_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.covenant_address, &QueryMsg::HolderAddress {})
            .unwrap()
    }
    #[allow(unused)]
    pub fn query_lp_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.covenant_address, &QueryMsg::LpAddress {})
            .unwrap()
    }
    #[allow(unused)]
    pub fn query_ls_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.covenant_address, &QueryMsg::LsAddress {})
            .unwrap()
    }
    #[allow(unused)]
    pub fn query_depositor_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.covenant_address, &QueryMsg::DepositorAddress {})
            .unwrap()
    }
}
