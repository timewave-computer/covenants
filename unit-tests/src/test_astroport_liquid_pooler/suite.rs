use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use covenant_astroport_liquid_pooler::msg::{LpConfig, ProvidedLiquidityInfo, QueryMsg};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, suite_builder::SuiteBuilder, CustomApp, ADMIN, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, TWO_PARTY_HOLDER_SALT};


pub(super) struct Suite {
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

impl Suite {
    fn build(
        mut builder: SuiteBuilder,
        liquid_pooler_addr: Addr,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_addr.clone(),
                &QueryMsg::ClockAddress {},
            )
            .unwrap();

        let holder_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_addr.clone(),
                &QueryMsg::HolderAddress {},
            )
            .unwrap();

        let lp_config = builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_addr.clone(),
                &QueryMsg::LpConfig {},
            )
            .unwrap();

        let provided_liquidity_info = builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_addr.clone(),
                &QueryMsg::ProvidedLiquidityInfo {},
            )
            .unwrap();

        Self {
            app: builder.app,
            faucet: builder.fuacet,
            admin: builder.admin,
            liquid_pooler_addr,
            clock_addr,
            holder_addr,
            lp_config,
            provided_liquidity_info,
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

        let clock_addr = builder.get_contract_addr(
            builder.clock_code_id,
            CLOCK_SALT,
        );
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

        Self::build(
            builder,
            liquid_pooler_addr,
        )
    }
}
