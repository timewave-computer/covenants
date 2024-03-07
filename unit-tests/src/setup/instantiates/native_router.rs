use std::collections::BTreeSet;

use cosmwasm_std::Addr;

use crate::setup::{suite_builder::SuiteBuilder, DENOM_ATOM_ON_NTRN};


pub struct NativeRouterInstantiate {
    pub msg: covenant_native_router::msg::InstantiateMsg,
}

impl From<NativeRouterInstantiate> for covenant_native_router::msg::InstantiateMsg {
    fn from(value: NativeRouterInstantiate) -> Self {
        value.msg
    }
}

impl NativeRouterInstantiate {
    pub fn new(
        clock_address: Addr,
        receiver_address: Addr,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self {
            msg: covenant_native_router::msg::InstantiateMsg {
                clock_address: clock_address.to_string(),
                receiver_address: receiver_address.to_string(),
                denoms,
            }
        }
    }

    pub fn with_clock_address(&mut self, addr: String) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_receiver_address(&mut self, addr: String) -> &mut Self {
        self.msg.receiver_address = addr;
        self
    }

    pub fn with_denoms(&mut self, denoms: BTreeSet<String>) -> &mut Self {
        self.msg.denoms = denoms;
        self
    }
}

impl NativeRouterInstantiate {
    pub fn default(
        clock_address: Addr,
        receiver_address: Addr,
    ) -> Self {
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        Self::new(
            clock_address,
            receiver_address,
            denoms,
        )
    }
}
