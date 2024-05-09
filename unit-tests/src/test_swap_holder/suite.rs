use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::swap_holder::SwapHolderInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NATIVE_SPLITTER_SALT,
    SWAP_HOLDER_SALT,
};
use cosmwasm_std::{Addr, Decimal};
use covenant_utils::{split::SplitConfig, CovenantPartiesConfig, CovenantTerms};
use cw_utils::Expiration;
use valence_swap_holder::msg::RefundConfig;

pub struct SwapHolderBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: SwapHolderInstantiate,
}

impl Default for SwapHolderBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let holder_addr = builder.get_contract_addr(builder.swap_holder_code_id, SWAP_HOLDER_SALT);
        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let native_splitter_addr =
            builder.get_contract_addr(builder.native_splitter_code_id, NATIVE_SPLITTER_SALT);

        let party_a_controller_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

        let party_a_router_addr =
            builder.get_contract_addr(builder.native_router_code_id, "party_a");
        let party_b_router_addr =
            builder.get_contract_addr(builder.native_router_code_id, "party_b");

        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![
                holder_addr.to_string(),
                native_splitter_addr.to_string(),
                party_a_router_addr.to_string(),
                party_b_router_addr.to_string(),
            ],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );
        let denom_set = BTreeSet::from_iter(vec![
            DENOM_ATOM_ON_NTRN.to_string(),
            DENOM_LS_ATOM_ON_NTRN.to_string(),
        ]);
        builder.contract_init2(
            builder.native_router_code_id,
            "party_a",
            &valence_native_router::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                receiver_address: party_a_controller_addr.to_string(),
                denoms: denom_set.clone(),
            },
            &[],
        );
        builder.contract_init2(
            builder.native_router_code_id,
            "party_b",
            &valence_native_router::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                receiver_address: party_b_controller_addr.to_string(),
                denoms: denom_set.clone(),
            },
            &[],
        );

        let mut splits = BTreeMap::new();
        splits.insert(
            party_a_router_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        splits.insert(
            party_b_router_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        let native_splitter_instantiate_msg = valence_native_splitter::msg::InstantiateMsg {
            clock_address: clock_addr.to_string(),
            splits: denom_to_split_config_map,
            fallback_split: None,
        };

        builder.contract_init2(
            builder.native_splitter_code_id,
            NATIVE_SPLITTER_SALT,
            &native_splitter_instantiate_msg,
            &[],
        );

        let holder_instantiate_msg = SwapHolderInstantiate::default(
            clock_addr.to_string(),
            native_splitter_addr.to_string(),
            party_a_controller_addr,
            party_b_controller_addr,
            party_a_router_addr.to_string(),
            party_b_router_addr.to_string(),
        );

        Self {
            builder,
            instantiate_msg: holder_instantiate_msg,
        }
    }
}

#[allow(dead_code)]
impl SwapHolderBuilder {
    pub fn with_clock_address(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_clock_address(addr);
        self
    }

    pub fn with_next_contract(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_next_contract(addr);
        self
    }

    pub fn with_lockup_config(mut self, period: Expiration) -> Self {
        self.instantiate_msg.with_lockup_config(period);
        self
    }

    pub fn with_covenant_terms(mut self, terms: CovenantTerms) -> Self {
        self.instantiate_msg.with_covenant_terms(terms);
        self
    }

    pub fn with_parties_config(mut self, config: CovenantPartiesConfig) -> Self {
        self.instantiate_msg.with_parties_config(config);
        self
    }

    pub fn build(mut self) -> Suite {
        let holder_addr = self.builder.contract_init2(
            self.builder.swap_holder_code_id,
            SWAP_HOLDER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &valence_swap_holder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let lockup_config = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &valence_swap_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap();

        let next_contract = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &valence_swap_holder::msg::QueryMsg::NextContract {},
            )
            .unwrap();

        let covenant_parties_config = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &valence_swap_holder::msg::QueryMsg::CovenantParties {},
            )
            .unwrap();

        let covenant_terms = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &valence_swap_holder::msg::QueryMsg::CovenantTerms {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr,
            holder: holder_addr,
            lockup_config,
            next_contract,
            covenant_parties_config,
            covenant_terms,
            emergency_committee_addr: None,
            app: self.builder.build(),
        }
    }
}

impl Suite {
    pub fn expire_lockup_config(&mut self) {
        let lockup_config = self.lockup_config;
        let app = self.get_app();
        match lockup_config {
            Expiration::AtHeight(h) => app.update_block(|b| b.height = h),
            Expiration::AtTime(t) => app.update_block(|b| b.time = t),
            Expiration::Never {} => (),
        };
    }

    pub fn query_next_contract(&self) -> Addr {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::NextContract {},
            )
            .unwrap()
    }

    pub fn query_lockup_config(&self) -> Expiration {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap()
    }

    pub fn query_covenant_parties_config(&self) -> CovenantPartiesConfig {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::CovenantParties {},
            )
            .unwrap()
    }

    pub fn query_covenant_terms(&self) -> CovenantTerms {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::CovenantTerms {},
            )
            .unwrap()
    }

    pub fn query_clock_address(&self) -> Addr {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap()
    }

    pub fn query_contract_state(&self) -> valence_swap_holder::msg::ContractState {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::ContractState {},
            )
            .unwrap()
    }

    pub fn query_deposit_address(&self) -> Option<Addr> {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::DepositAddress {},
            )
            .unwrap()
    }

    pub fn query_refund_config(&self) -> RefundConfig {
        self.get_app()
            .wrap()
            .query_wasm_smart(
                self.holder.clone(),
                &valence_swap_holder::msg::QueryMsg::RefundConfig {},
            )
            .unwrap()
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub holder: Addr,
    pub clock_addr: Addr,
    pub lockup_config: Expiration,
    pub next_contract: Addr,
    pub covenant_parties_config: CovenantPartiesConfig,
    pub covenant_terms: CovenantTerms,
    pub emergency_committee_addr: Option<String>,
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
