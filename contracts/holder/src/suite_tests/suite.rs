use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppBuilder, AppResponse, Executor};

use super::holder_contract;

const ADMIN: &str = "admin";
pub const DEFAULT_WITHDRAWER: &str = "authorizedwithdrawer";

pub struct Suite {
    pub app: App,
    pub holder: Addr,
    pub admin: Addr,
    pub holder_code_id: u64,
    pub pool_address: String,
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                withdrawer: DEFAULT_WITHDRAWER.to_string(),
                lp_address: "stablepairpool".to_string(),
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_withdrawer(mut self, addr: String) -> Self {
        self.instantiate.withdrawer = addr;
        self
    }

    pub fn with_lp(mut self, addr: String) -> Self {
        self.instantiate.lp_address = addr;
        self
    }

    pub fn build(self) -> Suite {
        let mut app = self.app;
        let holder_code = app.store_code(holder_contract());
        let holder = app
            .instantiate_contract(
                holder_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "holder",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        Suite {
            app,
            holder,
            admin: Addr::unchecked(ADMIN),
            holder_code_id: holder_code,
            pool_address: self.instantiate.lp_address,
        }
    }
}

// actions
impl Suite {
    /// sends a message on caller's behalf to withdraw a specified amount of tokens
    pub fn withdraw_tokens(&mut self, caller: &str, quantity: Vec<Coin>) -> AppResponse {
        self.app
            .execute_contract(
                Addr::unchecked(caller),
                self.holder.clone(),
                &ExecuteMsg::Withdraw {
                    quantity: Some(quantity),
                },
                &[],
            )
            .unwrap()
    }

    /// sends a message on caller's behalf to withdraw remaining balance
    pub fn withdraw_all(&mut self, caller: &str) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::Withdraw { quantity: None },
            &[],
        )
    }
}

// queries
impl Suite {
    pub fn query_withdrawer(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::Withdrawer {})
            .unwrap()
    }

    pub fn query_lp_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::LpAddress {})
            .unwrap()
    }
}

// helper
impl Suite {
    pub fn fund_holder(&mut self, tokens: Vec<Coin>) -> AppResponse {
        self.app
            .sudo(cw_multi_test::SudoMsg::Bank(
                cw_multi_test::BankSudo::Mint {
                    to_address: self.holder.to_string(),
                    amount: tokens,
                },
            ))
            .unwrap()
    }

    pub fn assert_holder_balance(&mut self, tokens: Vec<Coin>) {
        for c in &tokens {
            let queried_amount = self
                .app
                .wrap()
                .query_balance(self.holder.to_string(), c.denom.clone())
                .unwrap();
            assert_eq!(&queried_amount, c);
        }
    }

    pub fn assert_withdrawer_balance(&mut self, tokens: Vec<Coin>) {
        for c in &tokens {
            let queried_amount = self
                .app
                .wrap()
                .query_balance(DEFAULT_WITHDRAWER.to_string(), c.denom.clone())
                .unwrap();
            assert_eq!(&queried_amount, c);
        }
    }
}
