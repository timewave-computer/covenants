use cw_multi_test::App;

use crate::msg::InstantiateMsg;

pub const ST_ATOM_DENOM: &str = "stuatom";
pub const NATIVE_ATOM_DENOM: &str = "uatom";

pub(crate) struct Suite {
    pub app: App,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                ls_instantiate: covenant_ls::msg::InstantiateMsg {
                    autopilot_position: todo!(),
                    clock_address: todo!(),
                    stride_neutron_ibc_transfer_channel_id: todo!(),
                    neutron_stride_ibc_connection_id: todo!(),
                    lp_address: todo!(),
                    ls_denom: todo!(),
                },
                depositor_instantiate: covenant_depositor::msg::InstantiateMsg {
                    st_atom_receiver: todo!(),
                    atom_receiver: todo!(),
                    clock_address: todo!(),
                    gaia_neutron_ibc_transfer_channel_id: todo!(),
                    neutron_gaia_connection_id: todo!(),
                    gaia_stride_ibc_transfer_channel_id: todo!(),
                },
                lp_instantiate: covenant_lp::msg::InstantiateMsg {
                    lp_position: todo!(),
                    clock_address: todo!(),
                    holder_address: todo!(),
                },
                clock_code: todo!(),
                clock_instantiate: covenant_clock::msg::InstantiateMsg {
                    tick_max_gas: todo!(),
                },
                ls_code: todo!(),
                depositor_code: todo!(),
                lp_code: todo!(),
            },
        }
    }
}


impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = App::default();

        Suite {
            app,
        }
    }

    pub fn with_ls(mut self, instantiate_msg: covenant_ls::msg::InstantiateMsg) -> Self {
        self.instantiate.ls_instantiate = instantiate_msg;
        self
    }

    pub fn with_lp(mut self, instantiate_msg: covenant_lp::msg::InstantiateMsg) -> Self {
        self.instantiate.lp_instantiate = instantiate_msg;
        self
    }

    pub fn with_depositor(mut self, instantiate_msg: covenant_depositor::msg::InstantiateMsg) -> Self {
        self.instantiate.depositor_instantiate = instantiate_msg;
        self
    }
}

// assertion helpers
impl Suite {}

// queries
impl Suite {}