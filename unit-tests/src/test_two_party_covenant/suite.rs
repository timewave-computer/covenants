use std::collections::BTreeMap;

use cosmwasm_std::{coin, Addr, Uint64};
use covenant_two_party_pol::msg::Timeouts;
use covenant_utils::split::SplitConfig;
use cw_multi_test::{AppResponse, Executor};
use cw_utils::Expiration;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::two_party_covenant::TwoPartyCovenantInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, ADMIN, DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    DENOM_LS_ATOM_ON_STRIDE, HUB_STRIDE_CHANNEL, NTRN_HUB_CHANNEL, NTRN_STRIDE_CHANNEL,
    SINGLE_PARTY_COVENANT_SALT, TWO_PARTY_COVENANT_SALT,
};

pub struct TwoPartyCovenantBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: TwoPartyCovenantInstantiate,
}

impl Default for TwoPartyCovenantBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        // init astro pools
        let (pool_addr, _lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let party_a_addr = builder.get_random_addr();
        let party_b_addr = builder.get_random_addr();

        let instantiate_msg = TwoPartyCovenantInstantiate::default(
            &builder,
            party_a_addr.clone(),
            party_b_addr.clone(),
            pool_addr.clone(),
        );

        Self {
            builder,
            instantiate_msg,
        }
    }
}

#[allow(dead_code)]
impl TwoPartyCovenantBuilder {
    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.instantiate_msg.with_timeouts(timeouts);
        self
    }

    pub fn with_ibc_fee(
        mut self,
        preset_ibc_fee: covenant_two_party_pol::msg::PresetIbcFee,
    ) -> Self {
        self.instantiate_msg.with_ibc_fee(preset_ibc_fee);
        self
    }

    pub fn with_contract_codes(
        mut self,
        contract_codes: covenant_two_party_pol::msg::CovenantContractCodeIds,
    ) -> Self {
        self.instantiate_msg.with_contract_codes(contract_codes);
        self
    }

    pub fn with_clock_tick_max_gas(mut self, clock_tick_max_gas: Option<Uint64>) -> Self {
        self.instantiate_msg
            .with_clock_tick_max_gas(clock_tick_max_gas);
        self
    }

    pub fn with_lockup_config(mut self, lockup_config: Expiration) -> Self {
        self.instantiate_msg.with_lockup_config(lockup_config);
        self
    }

    pub fn with_ragequit_config(
        mut self,
        ragequit_config: Option<covenant_two_party_pol_holder::msg::RagequitConfig>,
    ) -> Self {
        self.instantiate_msg.with_ragequit_config(ragequit_config);
        self
    }

    pub fn with_deposit_deadline(mut self, deposit_deadline: Expiration) -> Self {
        self.instantiate_msg.with_deposit_deadline(deposit_deadline);
        self
    }

    pub fn with_party_a_config(
        mut self,
        party_a_config: covenant_two_party_pol::msg::CovenantPartyConfig,
    ) -> Self {
        self.instantiate_msg.with_party_a_config(party_a_config);
        self
    }

    pub fn with_party_b_config(
        mut self,
        party_b_config: covenant_two_party_pol::msg::CovenantPartyConfig,
    ) -> Self {
        self.instantiate_msg.with_party_b_config(party_b_config);
        self
    }

    pub fn with_covenant_type(
        mut self,
        covenant_type: covenant_two_party_pol_holder::msg::CovenantType,
    ) -> Self {
        self.instantiate_msg.with_covenant_type(covenant_type);
        self
    }

    pub fn with_party_a_share(mut self, party_a_share: Uint64) -> Self {
        self.instantiate_msg.with_party_a_share(party_a_share);
        self
    }

    pub fn with_party_b_share(mut self, party_b_share: Uint64) -> Self {
        self.instantiate_msg.with_party_b_share(party_b_share);
        self
    }

    pub fn with_pool_price_config(
        mut self,
        pool_price_config: covenant_utils::PoolPriceConfig,
    ) -> Self {
        self.instantiate_msg
            .with_pool_price_config(pool_price_config);
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

    pub fn with_emergency_committee(mut self, emergency_committee: Option<String>) -> Self {
        self.instantiate_msg
            .with_emergency_committee(emergency_committee);
        self
    }

    pub fn with_liquid_pooler_config(
        mut self,
        liquid_pooler_config: covenant_two_party_pol::msg::LiquidPoolerConfig,
    ) -> Self {
        self.instantiate_msg
            .with_liquid_pooler_config(liquid_pooler_config);
        self
    }

    pub fn build(mut self) -> Suite {
        let covenant_addr = self.builder.contract_init2(
            self.builder.two_party_covenant_code_id,
            TWO_PARTY_COVENANT_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let holder_addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::HolderAddress {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            covenant_addr,
            app: self.builder.build(),
            clock_addr,
            holder_addr,
        }
    }
}

pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub covenant_addr: Addr,
    pub clock_addr: Addr,
    pub holder_addr: Addr,
}

impl Suite {
    pub fn migrate_update(
        &mut self,
        code: u64,
        msg: covenant_two_party_pol::msg::MigrateMsg,
    ) -> AppResponse {
        self.app
            .migrate_contract(
                Addr::unchecked(ADMIN),
                self.covenant_addr.clone(),
                &msg,
                code,
            )
            .unwrap()
    }

    pub fn query_clock_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::ClockAddress {},
            )
            .unwrap()
    }

    pub fn query_holder_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::HolderAddress {},
            )
            .unwrap()
    }

    pub fn query_ibc_forwarder_address(&self, party: &str) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart::<Addr>(
                self.covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::IbcForwarderAddress {
                    party: party.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_liquid_pooler_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart::<Addr>(
                self.covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::LiquidPoolerAddress {},
            )
            .unwrap()
    }

    pub fn query_interchain_router_address(&self, party: &str) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart::<Addr>(
                self.covenant_addr.clone(),
                &covenant_two_party_pol::msg::QueryMsg::InterchainRouterAddress {
                    party: party.to_string(),
                },
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
