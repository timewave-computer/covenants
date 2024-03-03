use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_std::{Addr, Uint64};
use covenant_utils::DestinationConfig;

use crate::setup::{base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, INTERCHAIN_ROUTER_SALT, NTRN_HUB_CHANNEL};


pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub clock_addr: Addr,
    pub receiver_config: covenant_utils::DestinationConfig,
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
                &covenant_interchain_router::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let receiver_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                router.clone(),
                &covenant_interchain_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap();

        let denoms = builder
            .app
            .wrap()
            .query_wasm_smart(
                router.clone(),
                &covenant_interchain_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap();

        Self {
            app: builder.app,
            faucet: builder.fuacet,
            admin: builder.admin,
            clock_addr,
            denoms,
            receiver_config,
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
        let interchain_router_addr = builder.get_contract_addr(
            builder.interchain_router_code_id,
            INTERCHAIN_ROUTER_SALT,
        );

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![interchain_router_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_receiver = builder.get_random_addr();
        
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        let destination_config = DestinationConfig {
            local_to_destination_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
            destination_receiver_addr: party_receiver.to_string(),
            ibc_transfer_timeout: Uint64::new(1000),
            denom_to_pfm_map: BTreeMap::new(),
        };

        let interchain_router_instantiate_msg = covenant_interchain_router::msg::InstantiateMsg {
            clock_address: clock_addr.clone(),
            destination_config,
            denoms,
        };

        builder.contract_init2(
            builder.interchain_router_code_id,
            INTERCHAIN_ROUTER_SALT,
            &interchain_router_instantiate_msg,
            &[],
        );

        Self::build(
            builder,
            interchain_router_addr,
        )
    }
}