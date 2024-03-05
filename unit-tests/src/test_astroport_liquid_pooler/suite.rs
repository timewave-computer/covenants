use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Coin, Decimal};
use covenant_astroport_liquid_pooler::msg::{LpConfig, ProvidedLiquidityInfo, QueryMsg};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut}, instantiates::astro_liquid_pooler::AstroLiquidPoolerInstantiate, suite_builder::SuiteBuilder, CustomApp, ADMIN, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, TWO_PARTY_HOLDER_SALT
};

pub struct AstroLiquidPoolerBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: AstroLiquidPoolerInstantiate,
}

impl Default for AstroLiquidPoolerBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        // init astro pools
        let (pool_addr, lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let liquid_pooler_addr = builder.get_contract_addr(
            builder.astro_pooler_code_id,
            ASTRO_LIQUID_POOLER_SALT,
        );
        let holder_addr = builder.get_random_addr();

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![liquid_pooler_addr.to_string()],
        };

        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let liquid_pooler_instantiate = AstroLiquidPoolerInstantiate::default(
            pool_addr.to_string(),
            clock_addr.to_string(),
            holder_addr.to_string(),
        );

        AstroLiquidPoolerBuilder {
            builder,
            instantiate_msg: liquid_pooler_instantiate,
        }
    }
}

impl AstroLiquidPoolerBuilder {
    pub fn with_custom_astroport_pool(mut self, pair_type: PairType, coin_a: Coin, coin_b: Coin) -> Self {
        let (pool_addr, lp_token_addr) = self.builder.init_astro_pool(
            pair_type,
            coin_a,
            coin_b,
        );
        self.instantiate_msg.with_pool_address(pool_addr.to_string());
        self
    }

    pub fn with_pool_address(mut self, pool_address: String) -> Self {
        self.instantiate_msg.with_pool_address(pool_address);
        self
    }

    pub fn with_clock_address(mut self, clock_address: String) -> Self {
        self.instantiate_msg.with_clock_address(clock_address);
        self
    }

    pub fn with_slippage_tolerance(mut self, slippage_tolerance: Option<Decimal>) -> Self {
        self.instantiate_msg.with_slippage_tolerance(slippage_tolerance);
        self
    }

    pub fn with_assets(mut self, assets: covenant_astroport_liquid_pooler::msg::AssetData) -> Self {
        self.instantiate_msg.with_assets(assets);
        self
    }

    pub fn with_single_side_lp_limits(mut self, single_side_lp_limits: SingleSideLpLimits) -> Self {
        self.instantiate_msg.with_single_side_lp_limits(single_side_lp_limits);
        self
    }

    pub fn with_pool_price_config(mut self, pool_price_config: PoolPriceConfig) -> Self {
        self.instantiate_msg.with_pool_price_config(pool_price_config);
        self
    }

    pub fn with_pair_type(mut self, pair_type: PairType) -> Self {
        self.instantiate_msg.with_pair_type(pair_type);
        self
    }

    pub fn with_holder_address(mut self, holder_address: String) -> Self {
        self.instantiate_msg.with_holder_address(holder_address);
        self
    }

    pub fn build(mut self) -> Suite {
        let liquid_pooler_address = self.builder.contract_init2(
            self.builder.astro_pooler_code_id,
            ASTRO_LIQUID_POOLER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr: Addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(liquid_pooler_address.to_string(), &QueryMsg::ClockAddress {})
            .unwrap();

        let holder_addr: Addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(liquid_pooler_address.to_string(), &QueryMsg::HolderAddress {})
            .unwrap();

        let lp_config: LpConfig = self.builder
            .app
            .wrap()
            .query_wasm_smart(liquid_pooler_address.to_string(), &QueryMsg::LpConfig {})
            .unwrap();

        let provided_liquidity_info: ProvidedLiquidityInfo = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_address.to_string(),
                &QueryMsg::ProvidedLiquidityInfo {},
            )
            .unwrap();

        let faucet = self.builder.faucet.clone();
        let admin = self.builder.admin.clone();

        Suite {
            faucet,
            admin,
            liquid_pooler_addr: liquid_pooler_address.clone(),
            clock_addr: clock_addr.clone(),
            holder_addr: holder_addr.clone(),
            lp_config: lp_config.clone(),
            provided_liquidity_info: provided_liquidity_info.clone(),
            app: self.builder.build(),
        }
    }
}

pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub liquid_pooler_addr: Addr,
    pub clock_addr: Addr,
    pub holder_addr: Addr,
    pub lp_config: LpConfig,
    pub provided_liquidity_info: ProvidedLiquidityInfo,
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
