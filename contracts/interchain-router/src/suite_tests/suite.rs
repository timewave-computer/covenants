use crate::{
    contract::execute,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    Addr, Coin, CosmosMsg, Empty, GovMsg, Uint64,
};
use covenant_utils::DestinationConfig;
use cw_multi_test::{
    App, AppResponse, BankKeeper, BasicAppBuilder, Contract, ContractWrapper, DistributionKeeper,
    Executor, FailingModule, Ibc, IbcAcceptingModule, StakeKeeper, WasmKeeper,
};

pub const ADMIN: &str = "admin";
pub const DEFAULT_RECEIVER: &str = "receiver";
pub const CLOCK_ADDR: &str = "clock";
pub const DEFAULT_CHANNEL: &str = "channel-1";

fn router_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_migrate(crate::contract::migrate),
    )
}

pub struct Suite {
    pub app: App<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        IbcAcceptingModule,
        FailingModule<GovMsg, Empty, Empty>,
    >,
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
                destination_chain_channel_id: DEFAULT_CHANNEL.to_string(),
                destination_receiver_addr: DEFAULT_RECEIVER.to_string(),
                ibc_transfer_timeout: Uint64::new(10),
            },
            app: App::default(),
        }
    }
}

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = BasicAppBuilder::default()
            .with_ibc(IbcAcceptingModule)
            .build(|_, _, _| ());

        let router_code = app.store_code(router_contract());

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

    pub fn query_destination_config(&self) -> DestinationConfig {
        self.app
            .wrap()
            .query_wasm_smart(&self.router, &QueryMsg::DestinationConfig {})
            .unwrap()
    }
}

// helper
impl Suite {
    pub fn fund_router(&mut self, tokens: Vec<Coin>) -> AppResponse {
        self.app
            .sudo(cw_multi_test::SudoMsg::Bank(
                cw_multi_test::BankSudo::Mint {
                    to_address: self.router.to_string(),
                    amount: tokens,
                },
            ))
            .unwrap()
    }

    pub fn assert_router_balance(&mut self, tokens: Vec<Coin>) {
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
