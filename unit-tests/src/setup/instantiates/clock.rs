use cosmwasm_std::Uint64;

use crate::setup::suite_builder::SuiteBuilder;

#[derive(Clone)]
pub struct ClockInstantiate {
    pub msg: covenant_clock::msg::InstantiateMsg,
}

impl From<ClockInstantiate> for covenant_clock::msg::InstantiateMsg {
    fn from(value: ClockInstantiate) -> Self {
        value.msg
    }
}

impl ClockInstantiate {
    pub fn new(tick_max_gas: Option<Uint64>, whitelist: Vec<String>) -> Self {
        Self {
            msg: covenant_clock::msg::InstantiateMsg {
                tick_max_gas,
                whitelist,
            },
        }
    }

    pub fn with_tick_max_gas(&mut self, gas: Uint64) -> &mut Self {
        self.msg.tick_max_gas = Some(gas);
        self
    }

    pub fn with_whitelist(&mut self, whitelist: Vec<String>) -> &mut Self {
        self.msg.whitelist = whitelist;
        self
    }
}

impl ClockInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        tick_max_gas: Option<Uint64>,
        whitelist: Vec<String>,
    ) -> Self {
        Self {
            msg: covenant_clock::msg::InstantiateMsg {
                tick_max_gas,
                whitelist,
            },
        }
    }
}
