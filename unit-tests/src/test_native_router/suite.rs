use std::collections::BTreeSet;

use cosmwasm_std::Addr;

use crate::setup::{
    base_suite::BaseSuiteMut, instantiates::native_router::NativeRouterInstantiate, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, NATIVE_ROUTER_SALT
};

pub struct NativeRouterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: NativeRouterInstantiate,
}

impl Default for NativeRouterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);

        let native_router_addr =
            builder.get_contract_addr(builder.native_router_code_id, NATIVE_ROUTER_SALT);

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![native_router_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_receiver = builder.get_random_addr();

        let native_router_instantiate = NativeRouterInstantiate::default(
            clock_addr,
            party_receiver,
        );

        Self {
            builder,
            instantiate_msg: native_router_instantiate,
        }
    }
}

#[allow(dead_code)]
impl NativeRouterBuilder {
    pub fn with_clock_address(mut self, addr: Addr) -> Self {
        self.instantiate_msg.with_clock_address(addr);
        self
    }

    pub fn with_receiver_address(mut self, addr: Addr) -> Self {
        self.instantiate_msg.with_receiver_address(addr);
        self
    }

    pub fn with_denoms(mut self, denoms: BTreeSet<String>) -> Self {
        self.instantiate_msg.with_denoms(denoms);
        self
    }

    pub fn build(mut self) -> Suite {
        let native_router_address = self.builder.contract_init2(
            self.builder.native_router_code_id,
            NATIVE_ROUTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &covenant_native_router::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let receiver_addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &covenant_native_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap();

        let denoms = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &covenant_native_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr,
            receiver_addr,
            denoms,
            app: self.builder.build(),
        }

    }
}

#[allow(dead_code)]
pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub clock_addr: Addr,
    pub receiver_addr: Addr,
    pub denoms: BTreeSet<String>,
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        self.clock_addr.clone()
    }

    fn get_faucet_addr(&mut self) -> Addr {
        self.faucet.clone()
    }
}
