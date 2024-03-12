use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{AppResponse, Executor};

use crate::setup::{
    base_suite::BaseSuiteMut, instantiates::osmo_lp_outpost::OsmpLpOutpostInstantiate,
    suite_builder::SuiteBuilder, CustomApp,
};

pub struct OsmoLpOutpostBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: OsmpLpOutpostInstantiate,
}

impl Default for OsmoLpOutpostBuilder {
    fn default() -> Self {
        Self {
            builder: SuiteBuilder::new(),
            instantiate_msg: OsmpLpOutpostInstantiate::default(),
        }
    }
}

impl OsmoLpOutpostBuilder {
    pub fn build(mut self) -> Suite {
        let outpost_addr = self.builder.contract_init(
            self.builder.osmo_lp_outpost_code_id,
            "outpost".to_string(),
            &self.instantiate_msg.msg,
            &[],
        );

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            outpost: outpost_addr,
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub outpost: Addr,
}

impl Suite {
    pub fn provide_liquidity(
        &mut self,
        funds: Vec<Coin>,
        sender: Addr,
        config: covenant_outpost_osmo_liquid_pooler::msg::OutpostProvideLiquidityConfig,
    ) -> AppResponse {
        self.app
            .execute_contract(
                sender,
                self.outpost.clone(),
                &covenant_outpost_osmo_liquid_pooler::msg::ExecuteMsg::ProvideLiquidity { config },
                &funds,
            )
            .unwrap()
    }

    pub fn withdraw_liquidity(
        &mut self,
        funds: Vec<Coin>,
        sender: Addr,
        config: covenant_outpost_osmo_liquid_pooler::msg::OutpostWithdrawLiquidityConfig,
    ) -> AppResponse {
        self.app
            .execute_contract(
                sender,
                self.outpost.clone(),
                &covenant_outpost_osmo_liquid_pooler::msg::ExecuteMsg::WithdrawLiquidity { config },
                &funds,
            )
            .unwrap()
    }
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        // outpost is not clocked
        Addr::unchecked("")
    }

    fn get_faucet_addr(&mut self) -> Addr {
        self.faucet.clone()
    }
}
