use std::collections::BTreeSet;

use cosmwasm_std::Addr;

use crate::setup::{base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, NATIVE_ROUTER_SALT};


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
}

impl Suite {
    pub fn build(
        mut builder: SuiteBuilder,
        router: Addr,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                router.clone(),
                &covenant_native_router::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let receiver_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                router.clone(),
                &covenant_native_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap();

        let denoms = builder
            .app
            .wrap()
            .query_wasm_smart(
                router.clone(),
                &covenant_native_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap();

        Self {
            app: builder.app,
            faucet: builder.fuacet,
            admin: builder.admin,
            clock_addr,
            receiver_addr,
            denoms,
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();
        
        let clock_addr = builder.get_contract_addr(
            builder.clock_code_id,
            CLOCK_SALT,
        );

        let native_router_addr = builder.get_contract_addr(
            builder.native_router_code_id,
            NATIVE_ROUTER_SALT,
        );

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
        
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        let native_router_instantiate_msg = covenant_native_router::msg::InstantiateMsg {
            clock_address: clock_addr.to_string(),
            receiver_address: party_receiver.to_string(),
            denoms: denoms.clone(),
        };

        builder.contract_init2(
            builder.native_router_code_id,
            NATIVE_ROUTER_SALT,
            &native_router_instantiate_msg,
            &[],
        );
        
        Self::build(builder, native_router_addr)
    }
}