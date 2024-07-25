use std::str::FromStr;

use astroport::{factory::PairType, native_coin_registry::CoinResponse};
use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};
use cw_multi_test::{App, Executor};

use crate::{msg::{AssetData, InstantiateMsg}, suite_tests::{astro_pair_custom_concentrated_contract, astro_pair_stable_contract, astro_pair_xyk_contract, astro_token_contract, astro_whitelist_contract}};

use super::{
    astro_coin_registry_contract, astro_factory_contract, holder_contract, liquid_pooler_contract, suite::{Suite, ADMIN, ATOM, CLOCK, NEUTRON}
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

        
    pub fn setup_astroport(&mut self) {
        let astroport_native_coin_registry_code = self.app.store_code(astro_coin_registry_contract()); 
        let astro_factory_code = self.app.store_code(astro_factory_contract());
        let astroport_pair_stable_code = self.app.store_code(astro_pair_stable_contract());
        let astroport_pair_concentrated_code = self.app.store_code(astro_pair_custom_concentrated_contract());
        let astroport_pair_xyk_code = self.app.store_code(astro_pair_xyk_contract());
        let astroport_whitelist_code = self.app.store_code(astro_whitelist_contract());
        let astroport_token_code = self.app.store_code(astro_token_contract());

        let token_addr = self.app
            .instantiate_contract(
                astroport_token_code,
                Addr::unchecked(ADMIN),
                &astroport::token::InstantiateMsg {
                    name: "nativetoken".to_string(),
                    symbol: "ntk".to_string(),
                    decimals: 5,
                    initial_balances: vec![],
                    mint: None,
                    marketing: None,
                },
                &[],
                "token",
                None,
            )
            .unwrap();

        println!("token_addr: {:?}", token_addr);

        let astroport_whitelist = self
            .app
            .instantiate_contract(
                astroport_whitelist_code,
                Addr::unchecked(ADMIN),
                &cw1_whitelist::msg::InstantiateMsg {
                    admins: vec![ADMIN.to_string()],
                    mutable: false,
                },
                &[],
                "whitelist",
                None,
            )
            .unwrap();

        println!("astroport_whitelist: {:?}", astroport_whitelist);

        let native_coins_registry = self
            .app
            .instantiate_contract(
                astroport_native_coin_registry_code,
                Addr::unchecked(ADMIN),
                &astroport::native_coin_registry::InstantiateMsg {
                    owner: ADMIN.to_string(),
                },
                &[],
                "coin_registry",
                None,
            )
            .unwrap();

        println!("native_coins_registry: {:?}", native_coins_registry);

        let register_coins_response = self.app
            .execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                native_coins_registry.clone(),
                &astroport::native_coin_registry::ExecuteMsg::Add {
                    native_coins: vec![(ATOM.to_string(), 6), (NEUTRON.to_string(), 6)],
                },
                &[],
            )
            .unwrap();
        println!("register_coins_response: {:?}", register_coins_response);

        let resp: Vec<CoinResponse> = self.app.wrap()
            .query_wasm_smart(
                native_coins_registry.clone(),
                &astroport::native_coin_registry::QueryMsg::NativeTokens { start_after: None, limit: None },
            )
            .unwrap();

        println!("coin registry query response: {:?}", resp);

        let astroport_factory = self.app.instantiate_contract(
            astro_factory_code,
            Addr::unchecked(ADMIN),
            &astroport::factory::InstantiateMsg {
                pair_configs: vec![
                    astroport::factory::PairConfig {
                        code_id: astroport_pair_xyk_code,
                        pair_type: astroport::factory::PairType::Xyk {},
                        total_fee_bps: 0,
                        maker_fee_bps: 0,
                        is_disabled: false,
                        is_generator_disabled: true,
                        permissioned: false,
                    },
                ],
                token_code_id: astroport_token_code,
                fee_address: None,
                generator_address: None,
                owner: ADMIN.to_string(),
                whitelist_code_id: astroport_whitelist_code,
                coin_registry_address: native_coins_registry.to_string(),
                tracker_config: None,
            },
            &[],
            "factory",
            None,
        ).unwrap();

        println!("\nastroport_factory: {:?}\n", astroport_factory);

        let xyk_init_params = to_json_binary(&astroport::pair::XYKPoolParams {
            track_asset_balances: None,
        })
        .unwrap();

        let create_pair_execute_msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type: PairType::Xyk {},
            asset_infos: vec![
                astroport::asset::AssetInfo::NativeToken {
                    denom: ATOM.to_string(),
                },
                astroport::asset::AssetInfo::NativeToken {
                    denom: NEUTRON.to_string(),
                },
            ],
            init_params: Some(xyk_init_params),
        };

        println!("create_pair_execute_msg: {:?}", create_pair_execute_msg);
        let astroport_pair_instance = self.app.execute_contract(
            Addr::unchecked(ADMIN.to_string()),
            astroport_factory.clone(),
            &create_pair_execute_msg,
            &[],
        ).unwrap();

        println!("astroport_pair_instance: {:?}", astroport_pair_instance);
    }

    pub fn build(mut self) -> Suite {
        let mut app = App::default();

        let holder_code = app.store_code(holder_contract());
        let liquid_pooler_code = app.store_code(liquid_pooler_contract());

        self.setup_astroport();

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

        // let liquid_pooler = app
        //     .instantiate_contract(
        //         liquid_pooler_code,
        //         Addr::unchecked(ADMIN),
        //         &instantiate_msg,
        //         &[],
        //         "liquid_pooler",
        //         Some(ADMIN.to_string()),
        //     )
        //     .unwrap();

        let liquidity_pool = Addr::unchecked("todo".to_string());

        Suite {
            app,
            astroport_tf_liquid_pooler: Addr::unchecked("todo".to_string()),
            liquidity_pool,
        }
    }
}
