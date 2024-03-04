use std::{collections::BTreeMap, str::FromStr};

use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use covenant_two_party_pol_holder::msg::{DenomSplits, TwoPartyPolCovenantParty};
use covenant_utils::{split::SplitConfig, PoolPriceConfig, SingleSideLpLimits};
use cw_utils::Expiration;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::two_party_pol_holder::TwoPartyHolderInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    TWO_PARTY_HOLDER_SALT,
};

pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub holder_addr: Addr,

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
        holder_addr: Addr,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let ragequit_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::RagequitConfig {},
            )
            .unwrap();

        let lockup_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap();

        let deposit_deadline = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::DepositDeadline {},
            )
            .unwrap();

        let covenant_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::Config {},
            )
            .unwrap();

        let denom_splits: DenomSplits = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::DenomSplits {},
            )
            .unwrap();

        let next_contract = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::NextContract {},
            )
            .unwrap();

        Self {
            clock_addr,
            next_contract,
            lockup_config,
            ragequit_config,
            deposit_deadline,
            covenant_config,
            splits: denom_splits.clone().explicit_splits,
            fallback_split: denom_splits.clone().fallback_split,
            emergency_committee_addr,
            faucet: builder.faucet.clone(),
            admin: builder.admin.clone(),
            holder_addr,
            app: builder.build(),
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();
        let holder_addr =
            builder.get_contract_addr(builder.two_party_holder_code_id, TWO_PARTY_HOLDER_SALT);
        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let liquid_pooler_addr =
            builder.get_contract_addr(builder.astro_pooler_code_id, ASTRO_LIQUID_POOLER_SALT);

        // init astro pools
        let (pool_addr, lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![holder_addr.to_string(), liquid_pooler_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let liquid_pooler_instantiate_msg = covenant_astroport_liquid_pooler::msg::InstantiateMsg {
            pool_address: pool_addr.to_string(),
            clock_address: clock_addr.to_string(),
            slippage_tolerance: None,
            assets: covenant_astroport_liquid_pooler::msg::AssetData {
                asset_a_denom: DENOM_ATOM_ON_NTRN.to_string(),
                asset_b_denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
            },
            single_side_lp_limits: SingleSideLpLimits {
                asset_a_limit: Uint128::new(100000),
                asset_b_limit: Uint128::new(100000),
            },
            pool_price_config: PoolPriceConfig {
                expected_spot_price: Decimal::one(),
                acceptable_price_spread: Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
            },
            pair_type: PairType::Stable {},
            holder_address: holder_addr.to_string(),
        };

        builder.contract_init2(
            builder.astro_pooler_code_id,
            ASTRO_LIQUID_POOLER_SALT,
            &liquid_pooler_instantiate_msg,
            &[],
        );

        let party_a_host_addr = builder.get_random_addr();
        let party_a_controller_addr = builder.get_random_addr();

        let party_b_host_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

        let mut splits = BTreeMap::new();
        splits.insert(
            party_a_controller_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        splits.insert(
            party_b_controller_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        let lockup_config = Expiration::AtHeight(100000);
        let deposit_deadline = Expiration::AtHeight(200000);
        let ragequit_config = covenant_two_party_pol_holder::msg::RagequitConfig::Disabled {};
        let fallback_split = None;
        let emergency_committee_addr = None;
        let covenant_config = covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig {
            party_a: TwoPartyPolCovenantParty {
                contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                host_addr: party_a_host_addr.to_string(),
                controller_addr: party_a_controller_addr.to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
                router: party_a_controller_addr.to_string(),
            },
            party_b: TwoPartyPolCovenantParty {
                contribution: coin(10_000, DENOM_LS_ATOM_ON_NTRN),
                host_addr: party_b_host_addr.to_string(),
                controller_addr: party_b_controller_addr.to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
                router: party_b_controller_addr.to_string(),
            },
            covenant_type: covenant_two_party_pol_holder::msg::CovenantType::Share {},
        };

        let holder_instantiate_msg = TwoPartyHolderInstantiate::default(
            &builder,
            clock_addr.to_string(),
            liquid_pooler_addr.to_string(),
            lockup_config,
            ragequit_config.clone(),
            deposit_deadline,
            covenant_config.clone(),
            denom_to_split_config_map.clone(),
            fallback_split.clone(),
            emergency_committee_addr.clone(),
        );

        builder.contract_init2(
            builder.two_party_holder_code_id,
            TWO_PARTY_HOLDER_SALT,
            &holder_instantiate_msg.msg,
            &[],
        );

        Self::build(builder, holder_addr, emergency_committee_addr)
    }
}
