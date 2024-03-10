use std::{collections::BTreeMap, str::FromStr};

use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use covenant_two_party_pol_holder::msg::{ContractState, DenomSplits, TwoPartyPolCovenantParty};
use covenant_utils::{split::SplitConfig, PoolPriceConfig, SingleSideLpLimits};
use cw_multi_test::{AppResponse, Executor};
use cw_utils::Expiration;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::two_party_pol_holder::TwoPartyHolderInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    TWO_PARTY_HOLDER_SALT,
};

pub struct TwoPartyHolderBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: TwoPartyHolderInstantiate,
}

impl Default for TwoPartyHolderBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();
        
        let holder_addr =
            builder.get_contract_addr(builder.two_party_holder_code_id, TWO_PARTY_HOLDER_SALT);
        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let liquid_pooler_addr =
            builder.get_contract_addr(builder.astro_pooler_code_id, ASTRO_LIQUID_POOLER_SALT);

        // init astro pools
        let (pool_addr, _lp_token_addr) = builder.init_astro_pool(
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

        let party_a_controller_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();
      

        let holder_instantiate_msg = TwoPartyHolderInstantiate::default(
            clock_addr.to_string(),
            liquid_pooler_addr.to_string(),
            party_a_controller_addr,
            party_b_controller_addr,
        );
        
        Self {
            builder,
            instantiate_msg: holder_instantiate_msg,
        }
    }
}

#[allow(dead_code)]
impl TwoPartyHolderBuilder {
    pub fn with_clock(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_clock(addr);
        self
    }

    pub fn with_next_contract(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_next_contract(addr);
        self
    }

    pub fn with_lockup_config(mut self, config: Expiration) -> Self {
        self.instantiate_msg.with_lockup_config(config);
        self
    }

    pub fn with_ragequit_config(mut self, config: covenant_two_party_pol_holder::msg::RagequitConfig) -> Self {
        self.instantiate_msg.with_ragequit_config(config);
        self
    }

    pub fn with_deposit_deadline(mut self, config: Expiration) -> Self {
        self.instantiate_msg.with_deposit_deadline(config);
        self
    }

    pub fn with_covenant_config(mut self, config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig) -> Self {
        self.instantiate_msg.with_covenant_config(config);
        self
    }

    pub fn with_splits(mut self, splits: BTreeMap<String, SplitConfig>) -> Self {
        self.instantiate_msg.with_splits(splits);
        self
    }
    
    pub fn with_fallback_split(mut self, split: SplitConfig) -> Self {
        self.instantiate_msg.with_fallback_split(split);
        self
    }

    pub fn with_emergency_committee(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_emergency_committee(addr);
        self
    }

    pub fn build(mut self) -> Suite {
        let holder_addr = self.builder.contract_init2(
            self.builder.two_party_holder_code_id,
            TWO_PARTY_HOLDER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let ragequit_config = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::RagequitConfig {},
            )
            .unwrap();

        let lockup_config = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap();

        let deposit_deadline = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::DepositDeadline {},
            )
            .unwrap();

        let covenant_config = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::Config {},
            )
            .unwrap();

        let denom_splits: DenomSplits = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::DenomSplits {},
            )
            .unwrap();

        let next_contract = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::NextContract {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            holder_addr,
            clock_addr,
            next_contract,
            lockup_config,
            ragequit_config,
            deposit_deadline,
            covenant_config,
            splits: denom_splits.clone().explicit_splits,
            fallback_split: denom_splits.clone().fallback_split,
            emergency_committee_addr: None, // todo after adding emergency committee query to holder contract
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
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

impl Suite {
    pub fn expire_deposit_deadline(&mut self) {
        let expiration = self.deposit_deadline;
        self.get_app().update_block(|b| match expiration {
            Expiration::AtHeight(h) => b.height = h,
            Expiration::AtTime(t) => b.time = t,
            Expiration::Never {  } => (),
        });
    }

    pub fn expire_lockup_config(&mut self) {
        let expiration = self.lockup_config;
        self.get_app().update_block(|b| match expiration {
            Expiration::AtHeight(h) => b.height = h,
            Expiration::AtTime(t) => b.time = t,
            Expiration::Never {  } => (),
        });
    }

    pub fn ragequit(&mut self, sender: &str) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::Ragequit { },
            &[],
        )
        .unwrap()
    }

    pub fn claim(&mut self, sender: &str) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::Claim { },
            &[],
        )
        .unwrap()
    }


    pub fn distribute(&mut self, sender: &str) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::Distribute { },
            &[],
        )
        .unwrap()
    }

    pub fn withdraw_failed(&mut self, sender: &str) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::WithdrawFailed { },
            &[],
        )
        .unwrap()
    }

    pub fn emergency_withdraw(&mut self, sender: &str) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::EmergencyWithdraw { },
            &[],
        )
        .unwrap()
    }

    pub fn distribute_fallback_split(&mut self, sender: &str, denoms: Vec<String>) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.holder_addr.clone(),
            &covenant_two_party_pol_holder::msg::ExecuteMsg::DistributeFallbackSplit { denoms },
            &[],
        )
        .unwrap()
    }

    pub fn query_contract_state(&mut self) -> ContractState {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_two_party_pol_holder::msg::QueryMsg::ContractState {},
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
