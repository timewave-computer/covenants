use cosmwasm_std::{Addr, Empty};
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
                label: todo!(),
                preset_ibc_fee: todo!(),
                timeouts: todo!(),
                contract_codes: todo!(),
                clock_tick_max_gas: todo!(),
                lockup_config: todo!(),
                covenant_terms: todo!(),
                party_a_config: todo!(),
                party_b_config: todo!(),
                splits: todo!(),
                fallback_split: todo!(),
            },
        }
    }
}

impl SuiteBuilder {
    pub fn build(mut self) -> Suite {
        let mut app = App::default();
        Suite {
            app,
            covenant_address: todo!(),
        }
    }
}

// assertion helpers
impl Suite {}

// queries
impl Suite {
  
}
