use std::{collections::BTreeMap, str::FromStr};

use crate::{
    msg::{
        ContractState, CovenantType, ExecuteMsg, InstantiateMsg, QueryMsg, RagequitConfig,
        TwoPartyPolCovenantConfig, TwoPartyPolCovenantParty,
    },
    suite_tests::mock_clock_deps_contract,
};
use cosmwasm_std::{Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
use covenant_clock::test_helpers::helpers::mock_clock_instantiate_message;
use covenant_utils::{SplitConfig, SplitType};
use cw_multi_test::{App, AppResponse, Executor, SudoMsg};
use cw_utils::Expiration;

use super::{
    mock_astro_lp_token_contract, mock_astro_pool_contract, mock_deposit_contract,
    two_party_pol_holder_contract,
};

pub const ADMIN: &str = "admin";

pub const DENOM_A: &str = "denom_a";
pub const DENOM_B: &str = "denom_b";

pub const PARTY_A_ADDR: &str = "party_a";
pub const PARTY_B_ADDR: &str = "party_b";

pub const PARTY_A_ROUTER: &str = "party_a_router";
pub const PARTY_B_ROUTER: &str = "party_b_router";

pub const CLOCK_ADDR: &str = "contract0";
pub const NEXT_CONTRACT: &str = "contract3";

pub const POOL: &str = "contract2";

pub const INITIAL_BLOCK_HEIGHT: u64 = 12345;
pub const INITIAL_BLOCK_NANOS: u64 = 1571797419879305533;

pub struct Suite {
    pub app: App,
    pub holder: Addr,
    pub mock_deposit: Addr,
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub app: App,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        let mut denom_a_split = BTreeMap::new();
        denom_a_split.insert(
            PARTY_A_ROUTER.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        denom_a_split.insert(
            PARTY_B_ROUTER.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        let mut denom_b_split = BTreeMap::new();
        denom_b_split.insert(
            PARTY_A_ROUTER.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        denom_b_split.insert(
            PARTY_B_ROUTER.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );

        let mut splits = BTreeMap::new();
        splits.insert(
            DENOM_A.to_string(),
            SplitType::Custom(SplitConfig {
                receivers: denom_a_split,
            }),
        );
        splits.insert(
            DENOM_B.to_string(),
            SplitType::Custom(SplitConfig {
                receivers: denom_b_split,
            }),
        );

        Self {
            instantiate: InstantiateMsg {
                pool_address: "contract2".to_string(),
                ragequit_config: RagequitConfig::Disabled,
                deposit_deadline: Expiration::Never {},
                clock_address: "contract0".to_string(),
                next_contract: "contract3".to_string(),
                lockup_config: Expiration::Never {},
                covenant_config: TwoPartyPolCovenantConfig {
                    party_a: TwoPartyPolCovenantParty {
                        router: PARTY_A_ROUTER.to_string(),
                        contribution: Coin {
                            denom: DENOM_A.to_string(),
                            amount: Uint128::new(200),
                        },
                        allocation: Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                        host_addr: PARTY_A_ADDR.to_string(),
                        controller_addr: PARTY_A_ADDR.to_string(),
                    },
                    party_b: TwoPartyPolCovenantParty {
                        router: PARTY_B_ROUTER.to_string(),
                        contribution: Coin {
                            denom: DENOM_B.to_string(),
                            amount: Uint128::new(200),
                        },
                        host_addr: PARTY_B_ADDR.to_string(),
                        controller_addr: PARTY_B_ADDR.to_string(),
                        allocation: Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                    },
                    covenant_type: crate::msg::CovenantType::Share {},
                },
                splits,
                fallback_split: None,
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_lockup_config(mut self, config: Expiration) -> Self {
        self.instantiate.lockup_config = config;
        self
    }

    pub fn with_covenant_config_type(mut self, config: CovenantType) -> Self {
        self.instantiate.covenant_config.covenant_type = config;
        self
    }

    pub fn with_splits(mut self, splits: BTreeMap<String, SplitType>) -> Self {
        self.instantiate.splits = splits;
        self
    }

    pub fn with_fallback_split(mut self, split: SplitConfig) -> Self {
        self.instantiate.fallback_split = Some(split);
        self
    }

    pub fn with_ragequit_config(mut self, config: RagequitConfig) -> Self {
        self.instantiate.ragequit_config = config;
        self
    }

    pub fn with_deposit_deadline(mut self, config: Expiration) -> Self {
        self.instantiate.deposit_deadline = config;
        self
    }

    pub fn with_allocations(mut self, a_allocation: Decimal, b_allocation: Decimal) -> Self {
        self.instantiate.covenant_config.party_a.allocation = a_allocation;
        self.instantiate.covenant_config.party_b.allocation = b_allocation;
        self
    }

    pub fn build(mut self) -> Suite {
        let mut app = self.app;
        let holder_code = app.store_code(two_party_pol_holder_contract());
        let mock_deposit_code = app.store_code(mock_deposit_contract());
        let astro_pool_mock_code = app.store_code(mock_astro_pool_contract());
        let astro_lp_token_mock_code = app.store_code(mock_astro_lp_token_contract());
        let clock_mock_code = app.store_code(mock_clock_deps_contract());

        let clock = app
            .instantiate_contract(
                clock_mock_code,
                Addr::unchecked(ADMIN),
                &mock_clock_instantiate_message(),
                &[],
                "clock_mock",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        self.instantiate.clock_address = clock.to_string();
        println!("clock address: {:?}", clock.to_string());
        let astro_lp = app
            .instantiate_contract(
                astro_lp_token_mock_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "astro_mock_lp_code",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        println!("astro lp address: {:?}", astro_lp.to_string());

        let denom_b = Coin {
            denom: DENOM_B.to_string(),
            amount: Uint128::new(500),
        };
        let denom_a = Coin {
            denom: DENOM_A.to_string(),
            amount: Uint128::new(500),
        };
        app.sudo(SudoMsg::Bank(cw_multi_test::BankSudo::Mint {
            to_address: astro_lp.to_string(),
            amount: vec![denom_a, denom_b],
        }))
        .unwrap();

        let astro_mock = app
            .instantiate_contract(
                astro_pool_mock_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "astro_mock",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        println!("astro_mock address: {:?}", astro_mock.to_string());

        self.instantiate.pool_address = astro_mock.to_string();

        let mock_deposit = app
            .instantiate_contract(
                mock_deposit_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "mock_deposit",
                Some(ADMIN.to_string()),
            )
            .unwrap();

        println!("mock deposit: {:?}", mock_deposit.to_string());
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
        println!("holder address: {holder:?}");

        Suite {
            app,
            holder,
            mock_deposit,
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

    pub fn distribute_fallback(
        &mut self,
        caller: &str,
        denoms: Vec<String>,
    ) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::DistributeFallbackSplit { denoms },
            &[],
        )
    }

    pub fn rq(&mut self, caller: &str) -> Result<AppResponse, anyhow::Error> {
        println!("hq holder {:?}", self.holder.to_string());
        let resp = self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::Ragequit {},
            &[],
        );
        println!("rq response: {resp:?}");
        resp
    }

    pub fn claim(&mut self, caller: &str) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.holder.clone(),
            &ExecuteMsg::Claim {},
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

    pub fn query_covenant_config(&self) -> TwoPartyPolCovenantConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_pool(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::PoolAddress {})
            .unwrap()
    }

    pub fn query_party_a(&self) -> TwoPartyPolCovenantParty {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::ConfigPartyA {})
            .unwrap()
    }

    pub fn query_party_b(&self) -> TwoPartyPolCovenantParty {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::ConfigPartyB {})
            .unwrap()
    }

    pub fn query_deposit_deadline(&self) -> Expiration {
        self.app
            .wrap()
            .query_wasm_smart(&self.holder, &QueryMsg::DepositDeadline {})
            .unwrap()
    }

    pub fn query_lockup_config(&self) -> Expiration {
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
        self.app.update_block(|b| b.height += n);
    }

    pub fn pass_minutes(&mut self, n: u64) {
        self.app.update_block(|b| b.time = b.time.plus_minutes(n));
    }

    pub fn fund_coin(&mut self, coin: Coin) -> AppResponse {
        self.app
            .sudo(SudoMsg::Bank(cw_multi_test::BankSudo::Mint {
                to_address: self.holder.to_string(),
                amount: vec![coin],
            }))
            .unwrap()
    }

    pub fn get_denom_a_balance(&mut self, addr: String) -> Uint128 {
        self.app.wrap().query_balance(addr, DENOM_A).unwrap().amount
    }

    pub fn get_denom_b_balance(&mut self, addr: String) -> Uint128 {
        self.app.wrap().query_balance(addr, DENOM_B).unwrap().amount
    }

    pub fn get_all_balances(&mut self, addr: String) -> Vec<Coin> {
        self.app.wrap().query_all_balances(addr).unwrap()
    }

    pub fn get_party_a_coin(&mut self, amount: Uint128) -> Coin {
        Coin {
            denom: DENOM_A.to_string(),
            amount,
        }
    }

    pub fn get_party_b_coin(&mut self, amount: Uint128) -> Coin {
        Coin {
            denom: DENOM_B.to_string(),
            amount,
        }
    }

    pub fn get_coin(&mut self, denom: String, amount: Uint128) -> Coin {
        Coin { denom, amount }
    }
}

pub fn get_default_block_info() -> BlockInfo {
    BlockInfo {
        height: 12345,
        time: Timestamp::from_nanos(1571797419879305533),
        chain_id: "cosmos-testnet-14002".to_string(),
    }
}
