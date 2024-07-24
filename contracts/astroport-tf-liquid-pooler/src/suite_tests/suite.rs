use crate::msg::{ContractState, ExecuteMsg, LpConfig, QueryMsg};
use cosmwasm_std::Addr;
use cw_multi_test::{App, AppResponse, Executor};

pub const ADMIN: &str = "admin";
pub const CLOCK: &str = "clock";
pub const ATOM: &str = "uatom";
pub const NEUTRON: &str = "untrn";

pub struct Suite {
    pub app: App,
    pub astroport_tf_liquid_pooler: Addr,
    pub liquidity_pool: Addr,
}

// actions
impl Suite {
    pub fn tick(&mut self, caller: &str) -> AppResponse {
        self.app
            .execute_contract(
                Addr::unchecked(caller),
                self.astroport_tf_liquid_pooler.clone(),
                &ExecuteMsg::Tick {},
                &[],
            )
            .unwrap()
    }
}

// queries
impl Suite {
    pub fn query_clock_addr(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.astroport_tf_liquid_pooler, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_contract_state(&self) -> ContractState {
        self.app
            .wrap()
            .query_wasm_smart(
                &self.astroport_tf_liquid_pooler,
                &QueryMsg::ContractState {},
            )
            .unwrap()
    }

    pub fn query_lp_config(&self) -> LpConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.astroport_tf_liquid_pooler, &QueryMsg::LpConfig {})
            .unwrap()
    }
}
