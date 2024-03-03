use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::DestinationConfig;

use crate::setup::suite_builder::SuiteBuilder;

pub struct InterchainRouterInstantiate {
    pub msg: covenant_interchain_router::msg::InstantiateMsg,
}

impl From<InterchainRouterInstantiate> for covenant_interchain_router::msg::InstantiateMsg {
    fn from(value: InterchainRouterInstantiate) -> Self {
        value.msg
    }
}

impl InterchainRouterInstantiate {
    pub fn new(
        clock_address: Addr,
        destination_config: DestinationConfig,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self {
            msg: covenant_interchain_router::msg::InstantiateMsg {
                clock_address,
                destination_config,
                denoms,
            }
        }
    }

    pub fn with_clock_address(&mut self, addr: Addr) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_destination_config(&mut self, destination_config: DestinationConfig) -> &mut Self {
        self.msg.destination_config = destination_config;
        self
    }

    pub fn with_denoms(&mut self, denoms: BTreeSet<String>) -> &mut Self {
        self.msg.denoms = denoms;
        self
    }
}

impl InterchainRouterInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        clock_address: Addr,
        destination_config: DestinationConfig,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self::new(
            clock_address,
            destination_config,
            denoms,
        )
    }
}