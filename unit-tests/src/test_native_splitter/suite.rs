use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{Addr, Decimal};
use covenant_utils::split::SplitConfig;

use crate::setup::{
    base_suite::BaseSuiteMut, instantiates::native_splitter::NativeSplitterInstantiate, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NATIVE_SPLITTER_SALT
};

pub struct NativeSplitterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: NativeSplitterInstantiate,
}

impl Default for NativeSplitterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let native_splitter_addr =
            builder.get_contract_addr(builder.native_splitter_code_id, NATIVE_SPLITTER_SALT);

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![native_splitter_addr.to_string()],
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
            clock_addr,
            party_a_controller_addr.to_string(),
            party_b_controller_addr.to_string(),
        );

        Self {
            builder,
            instantiate_msg: native_splitter_instantiate,
        }
    }
}

#[allow(dead_code)]
impl NativeSplitterBuilder {
    pub fn with_clock_address(mut self, addr: Addr) -> Self {
        self.instantiate_msg.with_clock_address(addr);
        self
    }

    pub fn with_splits(mut self, splits: BTreeMap::<String, SplitConfig>) -> Self {
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

        let clock_addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &covenant_native_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let splits: Vec<(String, SplitConfig)> = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &covenant_native_splitter::msg::QueryMsg::Splits {},
            )
            .unwrap();

        let split_map = BTreeMap::from_iter(splits);

        let fallback_split = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                native_splitter_address.clone(),
                &covenant_native_splitter::msg::QueryMsg::FallbackSplit {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr,
            splits: split_map,
            fallback_split,
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
    pub splits: BTreeMap<String, SplitConfig>,
    pub fallback_split: Option<SplitConfig>,
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
