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
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                withdrawer: Some(DEFAULT_WITHDRAWER.to_string()),
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_withdrawer(mut self, w: Option<String>) -> Self {
        self.instantiate.withdrawer = w;
        self
    }

    pub fn with_funded_user(mut self, user: Addr, amount: Vec<Coin>) -> Self {
        self.app = AppBuilder::new().build(|router, _, storage| {
            router.bank.init_balance(storage, &user, amount).unwrap();
        });
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
        }
    }
}

// actions
impl Suite {
    /// sends a message on caller's behalf to withdraw a specified amount of tokens
    pub fn withdraw_tokens(
        &mut self,
        caller: &str,
        quantity: Vec<Coin>,
    ) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::Withdraw {
                quantity: Some(quantity),
            },
            &[],
        )
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
}

// helper
impl Suite {
    pub fn fund_holder(&mut self, funder: Addr, tokens: Vec<Coin>) -> anyhow::Result<AppResponse> {
        self.app.send_tokens(funder, self.holder.clone(), &tokens)
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
