use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{App, AppResponse, Executor, SudoMsg};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NativeReceiver, QueryMsg, ReceiverType, SplitConfig,
    SplitType,
};

use super::splitter_contract;

pub const ADMIN: &str = "admin";

pub const DENOM_A: &str = "denom_a";
pub const DENOM_B: &str = "denom_b";
pub const ALT_DENOM: &str = "alt_denom";

pub const PARTY_A_ADDR: &str = "party_a";
pub const PARTY_B_ADDR: &str = "party_b";

pub const CLOCK_ADDR: &str = "clock_addr";

pub fn get_equal_split_config() -> SplitConfig {
    SplitConfig {
        receivers: vec![
            (
                ReceiverType::Native(NativeReceiver {
                    address: PARTY_A_ADDR.to_string(),
                }),
                Uint128::new(50),
            ),
            (
                ReceiverType::Native(NativeReceiver {
                    address: PARTY_B_ADDR.to_string(),
                }),
                Uint128::new(50),
            ),
        ],
    }
}

pub fn get_fallback_split_config() -> SplitConfig {
    SplitConfig {
        receivers: vec![(
            ReceiverType::Native(NativeReceiver {
                address: "save_the_cats".to_string(),
            }),
            Uint128::new(100),
        )],
    }
}

pub struct Suite {
    pub app: App,
    pub splitter: Addr,
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                clock_address: CLOCK_ADDR.to_string(),
                splits: vec![
                    (
                        DENOM_A.to_string(),
                        SplitType::Custom(get_equal_split_config()),
                    ),
                    (
                        DENOM_B.to_string(),
                        SplitType::Custom(get_equal_split_config()),
                    ),
                ],
                fallback_split: None,
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_custom_splits(mut self, splits: Vec<(String, SplitType)>) -> Self {
        self.instantiate.splits = splits;
        self
    }

    pub fn with_fallback_split(mut self, split: SplitConfig) -> Self {
        self.instantiate.fallback_split = Some(SplitType::Custom(split));
        self
    }

    pub fn build(self) -> Suite {
        let mut app = self.app;

        let splitter_code: u64 = app.store_code(splitter_contract());
        let splitter = app
            .instantiate_contract(
                splitter_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "splitter",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        Suite { app, splitter }
    }
}

// actions
impl Suite {
    pub fn tick(&mut self, caller: &str) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.splitter.clone(),
            &ExecuteMsg::Tick {},
            &[],
        )
    }

    pub fn migrate(&mut self, msg: MigrateMsg) -> Result<AppResponse, anyhow::Error> {
        self.app
            .migrate_contract(Addr::unchecked(ADMIN), self.splitter.clone(), &msg, 1)
    }
}

// queries
impl Suite {
    pub fn query_clock_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.splitter, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_denom_split(&self, denom: String) -> SplitConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.splitter, &QueryMsg::DenomSplit { denom })
            .unwrap()
    }

    pub fn query_all_splits(&self) -> Vec<(String, SplitConfig)> {
        self.app
            .wrap()
            .query_wasm_smart(&self.splitter, &QueryMsg::Splits {})
            .unwrap()
    }

    pub fn query_fallback_split(&self) -> Option<SplitConfig> {
        self.app
            .wrap()
            .query_wasm_smart(&self.splitter, &QueryMsg::FallbackSplit {})
            .unwrap()
    }
}

// helper
impl Suite {
    pub fn pass_blocks(&mut self, n: u64) {
        self.app.update_block(|mut b| b.height += n);
    }

    pub fn fund_coin(&mut self, coin: Coin) -> AppResponse {
        self.app
            .sudo(SudoMsg::Bank(cw_multi_test::BankSudo::Mint {
                to_address: self.splitter.to_string(),
                amount: vec![coin],
            }))
            .unwrap()
    }

    pub fn get_party_denom_balance(&self, denom: &str, party_addr: &str) -> Uint128 {
        self.app
            .wrap()
            .query_balance(party_addr, denom)
            .unwrap()
            .amount
    }
}
