use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{Addr, Decimal};
use covenant_utils::split::SplitConfig;

use crate::setup::{base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NATIVE_SPLITTER_SALT};


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
}

impl Suite {
    pub fn build(
        mut builder: SuiteBuilder,
        splitter: Addr,
    ) -> Self {

        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_native_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let splits: Vec<(String, SplitConfig)> = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_native_splitter::msg::QueryMsg::Splits {},
            )
            .unwrap();

        let split_map = BTreeMap::from_iter(splits);


        let fallback_split = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_native_splitter::msg::QueryMsg::FallbackSplit {},
            )
            .unwrap();


        Self {
            faucet: builder.fuacet.clone(),
            admin: builder.admin.clone(),
            clock_addr,
            splits: split_map,
            fallback_split,
            app: builder.build(),
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
        let native_splitter_addr = builder.get_contract_addr(
            builder.native_splitter_code_id,
            NATIVE_SPLITTER_SALT,
        );

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

        let mut splits = BTreeMap::new();
        splits.insert(party_a_controller_addr.to_string(), Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_controller_addr.to_string(), Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig {
            receivers: splits,
        };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        let native_splitter_instantiate_msg = covenant_native_splitter::msg::InstantiateMsg {
            clock_address: clock_addr.clone(),
            splits: denom_to_split_config_map,
            fallback_split: None,
        };

        builder.contract_init2(
            builder.native_splitter_code_id,
            NATIVE_SPLITTER_SALT,
            &native_splitter_instantiate_msg,
            &[],
        );


        Self::build(
            builder,
            native_splitter_addr,
        )
    }
}
