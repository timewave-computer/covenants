use std::collections::BTreeSet;

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    Addr, Coin, Empty, GovMsg, Uint64,
};
use covenant_utils::{DestinationConfig, ReceiverConfig};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BasicAppBuilder, Contract, ContractWrapper, DistributionKeeper,
    Executor, FailingModule, IbcAcceptingModule, StakeKeeper, WasmKeeper,
};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use super::mock_clock_neutron_deps_contract;

pub const ADMIN: &str = "admin";
pub const DEFAULT_RECEIVER: &str = "receiver";
pub const CLOCK_ADDR: &str = "clock";
pub const DEFAULT_CHANNEL: &str = "channel-1";

fn router_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);

    Box::new(contract)
}

type CustomApp = App<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<NeutronMsg, NeutronQuery, Empty>,
    WasmKeeper<NeutronMsg, NeutronQuery>,
    StakeKeeper,
    DistributionKeeper,
    IbcAcceptingModule,
    FailingModule<GovMsg, Empty, Empty>,
>;

pub struct Suite {
    pub app: CustomApp,
    pub router: Addr,
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
                receiver_config: covenant_utils::ReceiverConfig::Ibc(DestinationConfig {
                    destination_chain_channel_id: DEFAULT_CHANNEL.to_string(),
                    destination_receiver_addr: DEFAULT_RECEIVER.to_string(),
                    ibc_transfer_timeout: Uint64::new(10),
                }),
                denoms: BTreeSet::new(),
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn with_denoms(mut self, denoms: Vec<String>) -> Self {
        let covenant_denoms: BTreeSet<String> = denoms.into_iter().collect();

        self.instantiate.denoms = covenant_denoms;
        self
    }

    pub fn build(self) -> Suite {
        let mut app = BasicAppBuilder::<NeutronMsg, NeutronQuery>::new_custom()
            .with_ibc(IbcAcceptingModule::new())
            .build(|_, _, _| ());

        let router_code = app.store_code(router_contract());
        let clock_code = app.store_code(mock_clock_neutron_deps_contract());

        self.instantiate.clock_address = app
            .instantiate_contract(
                clock_code,
                Addr::unchecked(ADMIN),
                &covenant_clock::msg::InstantiateMsg {
                    tick_max_gas: None,
                    whitelist: vec![],
                },
                &[],
                "clock",
                Some(ADMIN.to_string()),
            )
            .unwrap()
            .to_string();

        let router = app
            .instantiate_contract(
                router_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "router",
                Some(ADMIN.to_string()),
            )
            .unwrap();

        Suite { app, router }
    }
}

// actions
impl Suite {
    pub fn tick(&mut self, caller: &str) -> AppResponse {
        self.app
            .execute_contract(
                Addr::unchecked(caller),
                self.router.clone(),
                &ExecuteMsg::Tick {},
                &[],
            )
            .unwrap()
    }

    pub fn migrate(&mut self, msg: MigrateMsg) -> Result<AppResponse, anyhow::Error> {
        self.app
            .migrate_contract(Addr::unchecked(ADMIN), self.router.clone(), &msg, 1)
    }
}

// queries
impl Suite {
    pub fn query_clock_addr(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.router, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_destination_config(&self) -> ReceiverConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.router, &QueryMsg::ReceiverConfig {})
            .unwrap()
    }

    pub fn query_target_denoms(&self) -> BTreeSet<String> {
        self.app
            .wrap()
            .query_wasm_smart(&self.router, &QueryMsg::TargetDenoms {})
            .unwrap()
    }
}

// helper
impl Suite {
    pub fn _fund_router(&mut self, tokens: Vec<Coin>) -> AppResponse {
        self.app
            .sudo(cw_multi_test::SudoMsg::Bank(
                cw_multi_test::BankSudo::Mint {
                    to_address: self.router.to_string(),
                    amount: tokens,
                },
            ))
            .unwrap()
    }

    pub fn _assert_router_balance(&mut self, tokens: Vec<Coin>) {
        for c in &tokens {
            let queried_amount = self
                .app
                .wrap()
                .query_balance(self.router.to_string(), c.denom.clone())
                .unwrap();
            assert_eq!(&queried_amount, c);
        }
    }
}
