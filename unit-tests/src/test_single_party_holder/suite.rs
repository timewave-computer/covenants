
use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};
use cw_utils::Expiration;

use crate::setup::{base_suite::{BaseSuite, BaseSuiteMut}, instantiates::single_party_holder::SinglePartyHolderInstantiate, suite_builder::SuiteBuilder, CustomApp, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, SINGLE_PARTY_HOLDER_SALT, TWO_PARTY_HOLDER_SALT};



pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,

    pub holder_addr: Addr,
    pub withdraw_to: Option<Addr>,
    pub withdrawer: Option<Addr>,
    pub liquid_pooler_address: Addr,
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        // single party holder is not clocked
        Addr::unchecked("")
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
    ) -> Self {
        let liquid_pooler_address = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::PoolerAddress {},
            )
            .unwrap();

        let withdrawer = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::Withdrawer {},
            )
            .unwrap();

        let withdraw_to = builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::WithdrawTo {},
            )
            .unwrap();

        Self {
            app: builder.app,
            faucet: builder.fuacet,
            admin: builder.admin,
            holder_addr,
            withdraw_to,
            withdrawer,
            liquid_pooler_address,
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

        let holder_addr = builder.get_contract_addr(
            builder.single_party_holder_code_id,
            SINGLE_PARTY_HOLDER_SALT,
        );
        let liquid_pooler_addr = builder.get_contract_addr(
            builder.astro_pooler_code_id,
            ASTRO_LIQUID_POOLER_SALT,
        );
        let clock_addr = builder.get_contract_addr(
            builder.clock_code_id,
            CLOCK_SALT,
        );

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

        let lockup_config = Expiration::AtHeight(100000);

        let holder_instantiate_msg = SinglePartyHolderInstantiate::default(
            &builder,
            liquid_pooler_addr.to_string(),
            lockup_config,
            None,
            None,
            None,
        );

        builder.contract_init2(
            builder.single_party_holder_code_id,
            SINGLE_PARTY_HOLDER_SALT,
            &holder_instantiate_msg.msg,
            &[],
        );

        Self::build(builder, holder_addr)
    }
}
