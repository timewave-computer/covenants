use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use covenant_astroport_liquid_pooler::state::CLOCK_ADDRESS;
use covenant_two_party_pol_holder::msg::TwoPartyPolCovenantParty;
use covenant_utils::split::SplitConfig;
use cw_utils::Expiration;

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, instantiates::two_party_pol_holder::TwoPartyHolderInstantiate, suite_builder::SuiteBuilder, CustomApp, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, TWO_PARTY_HOLDER_SALT};


pub(super) struct Suite {
    pub app: CustomApp,
    pub clock_addr: Addr,
    pub next_contract: Addr,
    pub lockup_config: Expiration,
    pub ragequit_config: covenant_two_party_pol_holder::msg::RagequitConfig,
    pub deposit_deadline: Expiration,
    pub covenant_config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
    pub splits: BTreeMap<String, SplitConfig>,
    pub fallback_split: Option<SplitConfig>,
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

impl BaseSuite for Suite {
    fn get_app(&self) -> &CustomApp {
        &self.app
    }
}

impl Suite {
    fn build(
        mut builder: SuiteBuilder,
        clock_addr: Addr,
        next_contract: Addr,
        lockup_config: Expiration,
        ragequit_config: covenant_two_party_pol_holder::msg::RagequitConfig,
        deposit_deadline: Expiration,
        covenant_config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
        splits: BTreeMap<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        Self {
            app: builder.build(),
            clock_addr,
            next_contract,
            lockup_config,
            ragequit_config,
            deposit_deadline,
            covenant_config,
            splits,
            fallback_split,
            emergency_committee_addr,
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();

        // init astro pools
        let (pool_addr, lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let party_a_host_addr = builder.get_random_addr();
        let party_a_controller_addr = builder.get_random_addr();


        let party_b_host_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

        // TODO: update these to actual contract addresses
        let clock_address = builder.get_random_addr();
        let next_contract = builder.get_random_addr();

        let lockup_config = Expiration::AtHeight(1000);
        let deposit_deadline = Expiration::AtHeight(2000);
        let ragequit_config = covenant_two_party_pol_holder::msg::RagequitConfig::Disabled {};
        let splits = BTreeMap::new();
        let fallback_split = None;
        let emergency_committee_addr = None;
        let covenant_config = covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig {
            party_a: TwoPartyPolCovenantParty {
                contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                host_addr: party_a_host_addr.to_string(),
                controller_addr: party_a_controller_addr.to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
                router: "router".to_string(),
            },
            party_b: TwoPartyPolCovenantParty {
                contribution: coin(10_000, DENOM_LS_ATOM_ON_NTRN),
                host_addr: party_b_host_addr.to_string(),
                controller_addr: party_b_controller_addr.to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
                router: "router".to_string(),
            },
            covenant_type: covenant_two_party_pol_holder::msg::CovenantType::Share {},
        };



        let holder_instantiate_msg = TwoPartyHolderInstantiate::default(
            &builder,
            clock_address.to_string(),
            next_contract.to_string(),
            lockup_config,
            ragequit_config.clone(),
            deposit_deadline,
            covenant_config.clone(),
            splits.clone(),
            fallback_split.clone(),
            emergency_committee_addr.clone(),
        );

        builder.contract_init2(
            builder.two_party_holder_code_id,
            TWO_PARTY_HOLDER_SALT,
            &holder_instantiate_msg.msg,
            &[],
        );

        Self::build(
            builder,
            Addr::unchecked("clock"),
            Addr::unchecked("next_contract"),
            lockup_config,
            ragequit_config,
            deposit_deadline,
            covenant_config,
            splits,
            fallback_split,
            emergency_committee_addr,
        )
    }
}

