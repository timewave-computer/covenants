use std::str::FromStr;

use astroport::factory::PairType;
use cosmwasm_std::{Addr, Decimal, Uint128};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};
use cw_multi_test::{App, Executor};

use crate::msg::{AssetData, InstantiateMsg};

use super::{
    holder_contract, liquid_pooler_contract,
    suite::{Suite, ADMIN, ATOM, CLOCK, NEUTRON},
};

pub struct SuiteBuilder {
    pub pool_address: String,
    pub clock_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
    pub pool_price_config: PoolPriceConfig,
    pub pair_type: PairType,
    pub holder_address: String,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            app: App::default(),
            pool_address: "todo".to_string(),
            clock_address: CLOCK.to_string(),
            slippage_tolerance: None,
            assets: AssetData {
                asset_a_denom: ATOM.to_string(),
                asset_b_denom: NEUTRON.to_string(),
            },
            single_side_lp_limits: SingleSideLpLimits {
                asset_a_limit: Uint128::new(10_000),
                asset_b_limit: Uint128::new(10_000),
            },
            pool_price_config: PoolPriceConfig {
                expected_spot_price: Decimal::from_str("0.1").unwrap(),
                acceptable_price_spread: Decimal::from_str("0.01").unwrap(),
            },
            pair_type: PairType::Xyk {},
            holder_address: "todo".to_string(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_slippage_tolerance(mut self, slippage_tolerance: Option<Decimal>) -> Self {
        self.slippage_tolerance = slippage_tolerance;
        self
    }

    pub fn with_assets(mut self, assets: AssetData) -> Self {
        self.assets = assets;
        self
    }

    pub fn with_single_side_lp_limits(mut self, single_side_lp_limits: SingleSideLpLimits) -> Self {
        self.single_side_lp_limits = single_side_lp_limits;
        self
    }

    pub fn with_pool_price_config(mut self, pool_price_config: PoolPriceConfig) -> Self {
        self.pool_price_config = pool_price_config;
        self
    }

    pub fn with_pair_type(mut self, pair_type: PairType) -> Self {
        self.pair_type = pair_type;
        self
    }

    pub fn with_holder_address(mut self, holder_address: String) -> Self {
        self.holder_address = holder_address;
        self
    }

    pub fn build(mut self) -> Suite {
        let mut app = App::default();

        let holder_code = app.store_code(holder_contract());
        let liquid_pooler_code = app.store_code(liquid_pooler_contract());

        let instantiate_msg = InstantiateMsg {
            pool_address: self.pool_address,
            clock_address: self.clock_address,
            slippage_tolerance: self.slippage_tolerance,
            assets: self.assets,
            single_side_lp_limits: self.single_side_lp_limits,
            pool_price_config: self.pool_price_config,
            pair_type: self.pair_type,
            holder_address: self.holder_address,
        };

        let liquid_pooler = app
            .instantiate_contract(
                liquid_pooler_code,
                Addr::unchecked(ADMIN),
                &instantiate_msg,
                &[],
                "liquid_pooler",
                Some(ADMIN.to_string()),
            )
            .unwrap();

        let liquidity_pool = Addr::unchecked("todo".to_string());

        Suite {
            app,
            astroport_tf_liquid_pooler: liquid_pooler,
            liquidity_pool,
        }
    }
}
