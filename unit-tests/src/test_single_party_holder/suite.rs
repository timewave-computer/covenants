use astroport::factory::PairType;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};
use cw_multi_test::{AppResponse, Executor};
use cw_utils::Expiration;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::single_party_holder::SinglePartyHolderInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, ASTRO_LIQUID_POOLER_SALT, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN,
    SINGLE_PARTY_HOLDER_SALT,
};

pub struct SinglePartyHolderBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: SinglePartyHolderInstantiate,
}

impl Default for SinglePartyHolderBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        // init astro pools
        let (pool_addr, _lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let holder_addr = builder.get_contract_addr(
            builder.single_party_holder_code_id,
            SINGLE_PARTY_HOLDER_SALT,
        );
        let liquid_pooler_addr =
            builder.get_contract_addr(builder.astro_pooler_code_id, ASTRO_LIQUID_POOLER_SALT);
        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);

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

        let holder_instantiate_msg =
            SinglePartyHolderInstantiate::default(liquid_pooler_addr.to_string());

        Self {
            builder,
            instantiate_msg: holder_instantiate_msg,
        }
    }
}

#[allow(dead_code)]
impl SinglePartyHolderBuilder {
    pub fn with_withdrawer(mut self, addr: Option<String>) -> Self {
        self.instantiate_msg.with_withdrawer(addr);
        self
    }

    pub fn with_withdraw_to(mut self, addr: Option<String>) -> Self {
        self.instantiate_msg.with_withdraw_to(addr);
        self
    }

    pub fn with_emergency_committee_addr(mut self, addr: Option<String>) -> Self {
        self.instantiate_msg.with_emergency_committee_addr(addr);
        self
    }

    pub fn with_pooler_address(mut self, addr: &str) -> Self {
        self.instantiate_msg.with_pooler_address(addr);
        self
    }

    pub fn with_lockup_period(mut self, period: Expiration) -> Self {
        self.instantiate_msg.with_lockup_period(period);
        self
    }

    pub fn build(mut self) -> Suite {
        let holder_addr = self.builder.contract_init2(
            self.builder.single_party_holder_code_id,
            SINGLE_PARTY_HOLDER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let liquid_pooler_address: Addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::PoolerAddress {},
            )
            .unwrap();

        let clock_address = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                liquid_pooler_address.to_string(),
                &covenant_astroport_liquid_pooler::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let withdrawer = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::Withdrawer {},
            )
            .unwrap();

        let withdraw_to = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::WithdrawTo {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            holder_addr,
            clock: clock_address,
            withdraw_to,
            withdrawer,
            liquid_pooler_address,
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub clock: Addr,

    pub holder_addr: Addr,
    pub withdraw_to: Option<Addr>,
    pub withdrawer: Option<Addr>,
    pub liquid_pooler_address: Addr,
}

impl Suite {
    pub fn execute_claim(&mut self, sender: Addr) -> AppResponse {
        let holder = self.holder_addr.clone();

        self.app
            .execute_contract(
                sender,
                holder,
                &covenant_single_party_pol_holder::msg::ExecuteMsg::Claim {},
                &[],
            )
            .unwrap()
    }

    pub fn execute_distribute(&mut self, sender: Addr, funds: Vec<Coin>) -> AppResponse {
        let holder = self.holder_addr.clone();

        self.app
            .execute_contract(
                sender,
                holder,
                &covenant_single_party_pol_holder::msg::ExecuteMsg::Distribute {},
                &funds,
            )
            .unwrap()
    }

    pub fn execute_withdraw_failed(&mut self, sender: Addr) -> AppResponse {
        let holder = self.holder_addr.clone();

        self.app
            .execute_contract(
                sender,
                holder,
                &covenant_single_party_pol_holder::msg::ExecuteMsg::WithdrawFailed {},
                &[],
            )
            .unwrap()
    }

    pub fn execute_emergency_withdraw(&mut self, sender: Addr) -> AppResponse {
        let holder = self.holder_addr.clone();

        self.app
            .execute_contract(
                sender,
                holder,
                &covenant_single_party_pol_holder::msg::ExecuteMsg::EmergencyWithdraw {},
                &[],
            )
            .unwrap()
    }

    pub fn expire_lockup(&mut self) {
        let expiration = self.query_lockup_period();
        self.app.update_block(|b| match expiration {
            Expiration::AtHeight(h) => b.height = h + 1,
            Expiration::AtTime(t) => b.time = t,
            Expiration::Never {} => (),
        })
    }

    pub fn fund_contract_coins(&mut self, funds: Vec<Coin>, addr: Addr) {
        self.fund_contract(&funds, addr)
    }

    pub fn enter_pool(&mut self) -> AppResponse {
        let pooler = self.liquid_pooler_address.clone();
        let funds = vec![
            coin(1_000_000, DENOM_ATOM_ON_NTRN),
            coin(1_000_000, DENOM_LS_ATOM_ON_NTRN),
        ];
        let clock = self.clock.clone();
        self.fund_contract(&funds, pooler.clone());

        self.app
            .execute_contract(
                clock,
                pooler,
                &covenant_astroport_liquid_pooler::msg::ExecuteMsg::Tick {},
                &[],
            )
            .unwrap()
    }

    pub fn query_withdrawer(&mut self) -> Option<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::Withdrawer {},
            )
            .unwrap()
    }

    pub fn query_withdraw_to(&mut self) -> Option<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::WithdrawTo {},
            )
            .unwrap()
    }

    pub fn query_pooler_address(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::PoolerAddress {},
            )
            .unwrap()
    }

    pub fn query_emergency_committee(&mut self) -> Option<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::EmergencyCommitteeAddr {},
            )
            .unwrap()
    }

    pub fn query_lockup_period(&mut self) -> Expiration {
        self.app
            .wrap()
            .query_wasm_smart(
                self.holder_addr.clone(),
                &covenant_single_party_pol_holder::msg::QueryMsg::LockupConfig {},
            )
            .unwrap()
    }
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        // single party holder is not clocked
        Addr::unchecked("")
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
