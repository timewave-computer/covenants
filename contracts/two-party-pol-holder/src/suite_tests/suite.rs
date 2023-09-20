use crate::msg::{ContractState, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{Addr, Coin, Uint128};
use covenant_utils::{
    CovenantPartiesConfig, CovenantParty, LockupConfig, PolCovenantTerms,
};
use cw_multi_test::{App, AppResponse, Executor, SudoMsg};

use super::{mock_deposit_contract, two_party_pol_holder_contract};

pub const ADMIN: &str = "admin";

pub const DENOM_A: &str = "denom_a";
pub const DENOM_B: &str = "denom_b";

pub const PARTY_A_ADDR: &str = "party_a";
pub const PARTY_B_ADDR: &str = "party_b";

pub const CLOCK_ADDR: &str = "clock_address";
pub const NEXT_CONTRACT: &str = "next_contract";

pub const INITIAL_BLOCK_HEIGHT: u64 = 12345;
pub const INITIAL_BLOCK_NANOS: u64 = 1571797419879305533;

pub struct Suite {
    pub app: App,
    pub holder: Addr,
    pub mock_deposit: Addr,
    pub party_a: CovenantParty,
    pub party_b: CovenantParty,
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                pool_address: todo!(),
                ragequit_config: todo!(),
                deposit_deadline: None,
                clock_address: CLOCK_ADDR.to_string(),
                next_contract: NEXT_CONTRACT.to_string(),
                lockup_config: LockupConfig::None,
                parties_config: todo!(),
                covenant_terms: PolCovenantTerms {
                    party_a_amount: Uint128::new(400),
                    party_b_amount: Uint128::new(20),
                },
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_lockup_config(mut self, config: LockupConfig) -> Self {
        self.instantiate.lockup_config = config;
        self
    }

    pub fn build(mut self) -> Suite {
        let mut app = self.app;
        let holder_code = app.store_code(two_party_pol_holder_contract());
        let mock_deposit_code = app.store_code(mock_deposit_contract());

        let mock_deposit = app
            .instantiate_contract(
                mock_deposit_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "holder",
                Some(ADMIN.to_string()),
            )
            .unwrap();

        self.instantiate.next_contract = mock_deposit.to_string();

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
            mock_deposit,
            party_a: self.instantiate.parties_config.party_a,
            party_b: self.instantiate.parties_config.party_b,
        }
    }
}

// actions
impl Suite {
    pub fn tick(&mut self, caller: &str) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::Tick {},
            &[],
        )
    }
}

// queries
impl Suite {
    pub fn query_next_contract(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::NextContract {})
            .unwrap()
    }

    pub fn query_lockup_config(&self) -> LockupConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::LockupConfig {})
            .unwrap()
    }

    pub fn query_clock_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_contract_state(&self) -> ContractState {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::ContractState {})
            .unwrap()
    }
}

// helper
impl Suite {
    pub fn pass_blocks(&mut self, n: u64) {
        self.app.update_block(|mut b| b.height += n);
    }

    pub fn pass_minutes(&mut self, n: u64) {
        self.app
            .update_block(|mut b| b.time = b.time.plus_minutes(n));
    }

    pub fn fund_coin(&mut self, coin: Coin) -> AppResponse {
        self.app
            .sudo(SudoMsg::Bank(cw_multi_test::BankSudo::Mint {
                to_address: self.holder.to_string(),
                amount: vec![coin],
            }))
            .unwrap()
    }
}
