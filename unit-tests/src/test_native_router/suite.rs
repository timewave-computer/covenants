use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::op_mode::{ContractOperationMode, ContractOperationModeConfig};
use cw_multi_test::{AppResponse, Executor};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::native_router::NativeRouterInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, CLOCK_SALT, NATIVE_ROUTER_SALT,
};

pub struct NativeRouterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: NativeRouterInstantiate,
    pub clock_addr: Addr,
}

impl Default for NativeRouterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);

        let native_router_addr =
            builder.get_contract_addr(builder.native_router_code_id, NATIVE_ROUTER_SALT);

        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![],
            initial_queue: vec![native_router_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_receiver = builder.get_random_addr();

        let native_router_instantiate = NativeRouterInstantiate::default(
            ContractOperationModeConfig::Permissioned(vec![clock_addr.to_string()]),
            party_receiver,
        );

        Self {
            builder,
            instantiate_msg: native_router_instantiate,
            clock_addr,
        }
    }
}

#[allow(dead_code)]
impl NativeRouterBuilder {
    pub fn with_op_mode(mut self, op_mode_cfg: ContractOperationModeConfig) -> Self {
        self.instantiate_msg.with_op_mode(op_mode_cfg);
        self
    }

    pub fn with_receiver_address(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_receiver_address(addr.to_string());
        self
    }

    pub fn with_denoms(mut self, denoms: Vec<String>) -> Self {
        let denom_set = BTreeSet::from_iter(denoms);
        self.instantiate_msg.with_denoms(denom_set);
        self
    }

    pub fn build(mut self) -> Suite {
        let native_router_address = self.builder.contract_init2(
            self.builder.native_router_code_id,
            NATIVE_ROUTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let op_mode: ContractOperationMode = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &valence_native_router::msg::QueryMsg::OperationMode {},
            )
            .unwrap();

        let receiver_addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &valence_native_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap();

        let denoms = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_router_address.clone(),
                &valence_native_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap();

        Suite {
            router_addr: native_router_address,
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr: self.clock_addr,
            op_mode,
            receiver_addr,
            denoms,
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub router_addr: Addr,
    pub clock_addr: Addr,
    pub op_mode: ContractOperationMode,
    pub receiver_addr: Addr,
    pub denoms: BTreeSet<String>,
}

impl Suite {
    pub fn query_receiver_config(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.router_addr.clone(),
                &valence_native_router::msg::QueryMsg::ReceiverConfig {},
            )
            .unwrap()
    }

    pub(crate) fn query_op_mode(&mut self) -> ContractOperationMode {
        self.app
            .wrap()
            .query_wasm_smart(
                self.router_addr.clone(),
                &valence_native_router::msg::QueryMsg::OperationMode {},
            )
            .unwrap()
    }

    pub fn query_target_denoms(&mut self) -> BTreeSet<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.router_addr.clone(),
                &valence_native_router::msg::QueryMsg::TargetDenoms {},
            )
            .unwrap()
    }

    pub fn distribute_fallback(&mut self, denoms: Vec<String>) -> AppResponse {
        self.app
            .execute_contract(
                self.receiver_addr.clone(),
                self.router_addr.clone(),
                &valence_native_router::msg::ExecuteMsg::DistributeFallback { denoms },
                &[],
            )
            .unwrap()
    }
}

impl BaseSuite for Suite {
    fn get_app(&self) -> &CustomApp {
        &self.app
    }
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
