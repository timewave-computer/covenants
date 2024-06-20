use std::collections::BTreeMap;

use cosmwasm_std::Addr;
use covenant_utils::{
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    split::SplitConfig,
};
use cw_multi_test::{AppResponse, Executor};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::native_splitter::NativeSplitterInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, CLOCK_SALT, NATIVE_SPLITTER_SALT,
};

pub struct NativeSplitterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: NativeSplitterInstantiate,
    pub clock_addr: Addr,
}

impl Default for NativeSplitterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let native_splitter_addr =
            builder.get_contract_addr(builder.native_splitter_code_id, NATIVE_SPLITTER_SALT);

        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![native_splitter_addr.to_string()],
            initial_queue: vec![],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_a_controller_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

        let native_splitter_instantiate = NativeSplitterInstantiate::default(
            ContractOperationModeConfig::Permissioned(vec![clock_addr.to_string()]),
            party_a_controller_addr.to_string(),
            party_b_controller_addr.to_string(),
        );

        Self {
            builder,
            instantiate_msg: native_splitter_instantiate,
            clock_addr,
        }
    }
}

#[allow(dead_code)]
impl NativeSplitterBuilder {
    pub fn with_op_mode(mut self, op_mode_cfg: ContractOperationModeConfig) -> Self {
        self.instantiate_msg.with_op_mode(op_mode_cfg);
        self
    }

    pub fn with_splits(mut self, splits: BTreeMap<String, SplitConfig>) -> Self {
        self.instantiate_msg.with_splits(splits);
        self
    }

    pub fn with_fallback_split(mut self, fallback_split: Option<SplitConfig>) -> Self {
        self.instantiate_msg.with_fallback_split(fallback_split);
        self
    }

    pub fn build(mut self) -> Suite {
        let native_splitter_address = self.builder.contract_init2(
            self.builder.native_splitter_code_id,
            NATIVE_SPLITTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let op_mode: ContractOperationMode = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::OperationMode {},
            )
            .unwrap();

        let splits: Vec<(String, SplitConfig)> = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &valence_native_splitter::msg::QueryMsg::Splits {},
            )
            .unwrap();
        let config_1 = splits[0].clone().1.receivers;
        let receivers: Vec<String> = config_1.keys().cloned().collect();

        let split_map = BTreeMap::from_iter(splits);

        let fallback_split = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &valence_native_splitter::msg::QueryMsg::FallbackSplit {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr: self.clock_addr,
            op_mode,
            splitter: native_splitter_address,
            splits: split_map,
            fallback_split,
            app: self.builder.build(),
            receiver_1: Addr::unchecked(receivers[0].to_string()),
            receiver_2: Addr::unchecked(receivers[1].to_string()),
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub splitter: Addr,
    pub clock_addr: Addr,
    pub op_mode: ContractOperationMode,
    pub splits: BTreeMap<String, SplitConfig>,
    pub fallback_split: Option<SplitConfig>,
    pub receiver_1: Addr,
    pub receiver_2: Addr,
}

impl Suite {
    pub(crate) fn query_op_mode(&mut self) -> ContractOperationMode {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_native_splitter::msg::QueryMsg::OperationMode {},
            )
            .unwrap()
    }

    pub fn query_denom_split(&mut self, denom: String) -> SplitConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_native_splitter::msg::QueryMsg::DenomSplit { denom },
            )
            .unwrap()
    }

    pub fn query_all_splits(&mut self) -> BTreeMap<String, SplitConfig> {
        let splits: Vec<(String, SplitConfig)> = self
            .app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_native_splitter::msg::QueryMsg::Splits {},
            )
            .unwrap();
        BTreeMap::from_iter(splits)
    }

    pub fn query_fallback_split(&mut self) -> Option<SplitConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_native_splitter::msg::QueryMsg::FallbackSplit {},
            )
            .unwrap()
    }

    pub fn query_deposit_address(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_native_splitter::msg::QueryMsg::DepositAddress {},
            )
            .unwrap()
    }

    pub fn distribute_fallback(&mut self, denoms: Vec<String>) -> AppResponse {
        self.app
            .execute_contract(
                self.faucet.clone(),
                self.splitter.clone(),
                &valence_native_splitter::msg::ExecuteMsg::DistributeFallback { denoms },
                &[],
            )
            .unwrap()
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

impl BaseSuite for Suite {
    fn get_app(&self) -> &CustomApp {
        &self.app
    }
}
