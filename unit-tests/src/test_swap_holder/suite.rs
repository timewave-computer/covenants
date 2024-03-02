use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{Addr, Decimal, Uint128};
use covenant_utils::{split::SplitConfig, CovenantPartiesConfig, CovenantParty, CovenantTerms, ReceiverConfig, SwapCovenantTerms};
use cw_utils::Expiration;
use crate::setup::{base_suite::BaseSuiteMut, instantiates::swap_holder::SwapHolderInstantiate, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NATIVE_SPLITTER_SALT, SWAP_HOLDER_SALT};


pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

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
}

impl Suite {
    pub fn build(
        mut builder: SuiteBuilder,
        holder_addr: Addr,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_swap_holder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let lockup_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_swap_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap();

        let next_contract = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_swap_holder::msg::QueryMsg::NextContract {},
            )
            .unwrap();

        let covenant_parties_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_swap_holder::msg::QueryMsg::CovenantParties {},
            )
            .unwrap();

        let covenant_terms = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_swap_holder::msg::QueryMsg::CovenantTerms {},
            )
            .unwrap();

        Self {
            faucet: builder.fuacet.clone(),
            admin: builder.admin.clone(),
            clock_addr,
            lockup_config,
            next_contract,
            covenant_parties_config,
            covenant_terms,
            emergency_committee_addr,
            app: builder.build(),
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();

        let holder_addr = builder.get_contract_addr(
            builder.swap_holder_code_id,
            SWAP_HOLDER_SALT,
        );
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
            whitelist: vec![holder_addr.to_string(), native_splitter_addr.to_string()],
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

        let holder_instantiate_msg = SwapHolderInstantiate {
            msg: covenant_swap_holder::msg::InstantiateMsg {
                next_contract: native_splitter_addr.to_string(),
                covenant_terms: CovenantTerms::TokenSwap(SwapCovenantTerms{
                    party_a_amount: Uint128::new(100000),
                    party_b_amount: Uint128::new(100000),
                }),
                clock_address: clock_addr.to_string(),
                lockup_config: Expiration::AtHeight(1000000),
                parties_config: CovenantPartiesConfig {
                    party_a: CovenantParty {
                        addr: party_a_controller_addr.to_string(),
                        native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                        receiver_config: ReceiverConfig::Native(party_a_controller_addr),
                    },
                    party_b: CovenantParty {
                        addr: party_b_controller_addr.to_string(),
                        native_denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                        receiver_config: ReceiverConfig::Native(party_b_controller_addr),
                    },
                },
            }
        };

        builder.contract_init2(
            builder.swap_holder_code_id,
            SWAP_HOLDER_SALT,
            &holder_instantiate_msg.msg,
            &[],
        );

        Self::build(
            builder,
            holder_addr,
            None,
        )
    }
}
