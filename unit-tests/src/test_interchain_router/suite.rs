use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::{
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    DestinationConfig,
};

use crate::setup::{
    base_suite::BaseSuiteMut, instantiates::interchain_router::InterchainRouterInstantiate,
    suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, INTERCHAIN_ROUTER_SALT,
};

pub struct InterchainRouterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: InterchainRouterInstantiate,
    pub clock_addr: Addr,
}

impl Default for InterchainRouterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let interchain_router_addr =
            builder.get_contract_addr(builder.interchain_router_code_id, INTERCHAIN_ROUTER_SALT);

        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![],
            initial_queue: vec![interchain_router_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_receiver = builder.get_random_addr();

        let interchain_router_instantiate =
            InterchainRouterInstantiate::default(clock_addr.clone(), party_receiver.to_string());

        Self {
            builder,
            instantiate_msg: interchain_router_instantiate,
            clock_addr,
        }
    }
}

#[allow(dead_code)]
impl InterchainRouterBuilder {
    pub fn with_op_mode(mut self, op_mode: ContractOperationModeConfig) -> Self {
        self.instantiate_msg.with_op_mode(op_mode);
        self
    }

    pub fn with_destination_config(mut self, destination_config: DestinationConfig) -> Self {
        self.instantiate_msg
            .with_destination_config(destination_config);
        self
    }

    pub fn with_denoms(mut self, denoms: BTreeSet<String>) -> Self {
        self.instantiate_msg.with_denoms(denoms);
        self
    }

    pub fn build(mut self) -> Suite {
        let interchain_router_address = self.builder.contract_init2(
            self.builder.interchain_router_code_id,
            INTERCHAIN_ROUTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let op_mode: ContractOperationMode = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                interchain_router_address.clone(),
                &valence_interchain_router::msg::QueryMsg::OperationMode {},
            )
            .unwrap();

        let receiver_config = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                interchain_router_address.clone(),
                &valence_interchain_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap();

        let denoms = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                interchain_router_address.clone(),
                &valence_interchain_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr: self.clock_addr,
            op_mode,
            denoms,
            receiver_config,
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub clock_addr: Addr,
    pub op_mode: ContractOperationMode,
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

    fn get_faucet_addr(&mut self) -> Addr {
        self.faucet.clone()
    }
}
