use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    factory::{PairConfig, PairType},
    pair::{Cw20HookMsg, PoolResponse, SimulationResponse, StablePoolParams},
};

use cosmwasm_std::{
    testing::MockApi, to_binary, Addr, Coin, Decimal, Empty, MemoryStorage, QueryRequest, Uint128,
    Uint64, WasmQuery,
};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{
    App, AppResponse, BankKeeper, BankSudo, Contract, ContractWrapper, Executor, FailingModule,
    SudoMsg, WasmKeeper,
};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use crate::msg::{AssetData, InstantiateMsg, QueryMsg, SingleSideLpLimits, LpConfig};
use astroport::factory::InstantiateMsg as FactoryInstantiateMsg;
use astroport::native_coin_registry::InstantiateMsg as NativeCoinRegistryInstantiateMsg;
use astroport::pair::InstantiateMsg as PairInstantiateMsg;
use astroport::token::InstantiateMsg as TokenInstantiateMsg;
use cw1_whitelist::msg::InstantiateMsg as WhitelistInstantiateMsg;

pub const CREATOR_ADDR: &str = "creator";
pub const ST_ATOM_DENOM: &str = "stuatom";
pub const NATIVE_ATOM_DENOM: &str = "uatom";

fn astro_token() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            astroport_token::contract::execute,
            astroport_token::contract::instantiate,
            astroport_token::contract::query,
        )
        .with_migrate(astroport_token::contract::migrate),
    )
}

fn astro_whitelist() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        astroport_whitelist::contract::instantiate,
        astroport_whitelist::contract::instantiate,
        astroport_whitelist::contract::query,
    ))
}

fn astro_factory() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            astroport_factory::contract::execute,
            astroport_factory::contract::instantiate,
            astroport_factory::contract::query,
        )
        .with_migrate(astroport_factory::contract::migrate)
        .with_reply(astroport_factory::contract::reply),
    )
}

fn astro_pair_stable() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            astroport_pair_stable::contract::execute,
            astroport_pair_stable::contract::instantiate,
            astroport_pair_stable::contract::query,
        )
        .with_reply(astroport_pair_stable::contract::reply)
        .with_migrate(astroport_pair_stable::contract::migrate),
    )
}

fn astro_coin_registry() -> Box<dyn Contract<Empty>> {
    let registry_contract = ContractWrapper::new(
        astroport_native_coin_registry::contract::execute,
        astroport_native_coin_registry::contract::instantiate,
        astroport_native_coin_registry::contract::query,
    )
    .with_migrate(astroport_native_coin_registry::contract::migrate);

    Box::new(registry_contract)
}

fn lper_contract() -> Box<dyn Contract<Empty>> {
    let lp_contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    // .with_migrate(crate::contract::migrate);

    Box::new(lp_contract)
}

fn holder_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            covenant_holder::contract::execute,
            covenant_holder::contract::instantiate,
            covenant_holder::contract::query,
        )
        .with_migrate(covenant_holder::contract::migrate),
    )
}

fn clock_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            covenant_clock::contract::execute,
            covenant_clock::contract::instantiate,
            covenant_clock::contract::query,
        )
        .with_reply(covenant_clock::contract::reply)
        .with_migrate(covenant_clock::contract::migrate),
    )
}

#[allow(unused)]
pub type BaseApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<NeutronMsg, NeutronQuery, Empty>,
    WasmKeeper<NeutronMsg, NeutronQuery>,
>;
#[allow(unused)]
pub(crate) struct Suite {
    pub app: App,
    pub admin: Addr,
    pub lp_token: Addr,
    // (token_code, contract_address)
    pub token: u64,
    pub whitelist: (u64, String),
    pub factory: (u64, String),
    pub stable_pair: (u64, String),
    pub coin_registry: (u64, String),
    pub liquid_pooler: (u64, String),
    pub clock_addr: String,
    pub holder_addr: String,
}

pub(crate) struct SuiteBuilder {
    pub lp_instantiate: InstantiateMsg,
    pub token_instantiate: TokenInstantiateMsg,
    pub whitelist_instantiate: WhitelistInstantiateMsg,
    pub factory_instantiate: FactoryInstantiateMsg,
    pub stablepair_instantiate: PairInstantiateMsg,
    pub registry_instantiate: NativeCoinRegistryInstantiateMsg,
    pub clock_instantiate: covenant_clock::msg::InstantiateMsg,
    pub holder_instantiate: covenant_holder::msg::InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            lp_instantiate: InstantiateMsg {
                clock_address: "clock-addr".to_string(),
                pool_address: "lp-addr".to_string(),
                // deterministic based on instantiate sequence
                holder_address: "contract1".to_string(),
                slippage_tolerance: Some(Decimal::one()),
                autostake: Some(false),
                assets: AssetData {
                    native_asset_denom: "uatom".to_string(),
                    ls_asset_denom: "stuatom".to_string(),
                },
                single_side_lp_limits: SingleSideLpLimits {
                    native_asset_limit: Uint128::new(100),
                    ls_asset_limit: Uint128::new(100),
                },
                expected_ls_token_amount: Uint128::new(40000),
                allowed_return_delta: Uint128::new(10000),
                expected_native_token_amount: Uint128::new(40000),
            },
            token_instantiate: TokenInstantiateMsg {
                name: "nativetoken".to_string(),
                symbol: "ntk".to_string(),
                decimals: 20,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            whitelist_instantiate: WhitelistInstantiateMsg {
                admins: vec![CREATOR_ADDR.to_string()],
                mutable: false,
            },
            factory_instantiate: FactoryInstantiateMsg {
                pair_configs: vec![PairConfig {
                    code_id: u64::MAX,
                    pair_type: astroport::factory::PairType::Stable {},
                    total_fee_bps: 0,
                    maker_fee_bps: 0,
                    is_disabled: false,
                    is_generator_disabled: true,
                }],
                token_code_id: u64::MAX,
                fee_address: None,
                generator_address: None,
                owner: CREATOR_ADDR.to_string(),
                whitelist_code_id: u64::MAX,
                coin_registry_address: "TODO".to_string(),
            },
            stablepair_instantiate: PairInstantiateMsg {
                asset_infos: vec![
                    astroport::asset::AssetInfo::NativeToken {
                        denom: ST_ATOM_DENOM.to_string(),
                    },
                    astroport::asset::AssetInfo::NativeToken {
                        denom: NATIVE_ATOM_DENOM.to_string(),
                    },
                ],
                token_code_id: u64::MAX,
                factory_addr: "TODO".to_string(),
                init_params: Some(
                    to_binary(&StablePoolParams {
                        amp: 1000,
                        owner: Some(CREATOR_ADDR.to_string()),
                    })
                    .unwrap(),
                ),
            },
            registry_instantiate: NativeCoinRegistryInstantiateMsg {
                owner: CREATOR_ADDR.to_string(),
            },
            clock_instantiate: covenant_clock::msg::InstantiateMsg {
                tick_max_gas: Some(Uint64::new(50000)),
                // this is the lper, if any instantiate flow changes, this needs to be updated
                whitelist: vec!["contract9".to_string()],
            },
            holder_instantiate: covenant_holder::msg::InstantiateMsg {
                withdrawer: Some(CREATOR_ADDR.to_string()),
                // deterministic based on instantiate flow
                pool_address: "contract7".to_string(),
            },
        }
    }
}

#[allow(unused)]
impl SuiteBuilder {
    fn with_slippage_tolerance(mut self, decimal: Decimal) -> Self {
        self.lp_instantiate.slippage_tolerance = Some(decimal);
        self
    }

    fn with_autostake(mut self, autosake: Option<bool>) -> Self {
        self.lp_instantiate.autostake = autosake;
        self
    }

    fn with_assets(mut self, assets: AssetData) -> Self {
        self.lp_instantiate.assets = assets;
        self
    }

    fn with_token_instantiate_msg(mut self, msg: TokenInstantiateMsg) -> Self {
        self.token_instantiate = msg;
        self
    }

    pub fn build(mut self) -> Suite {
        // let mut app = BasicAppBuilder::<NeutronMsg, NeutronQuery>::new_custom().build(|_,_,_| {});

        let mut app = App::default();

        let token_code = app.store_code(astro_token());
        let stablepair_code = app.store_code(astro_pair_stable());
        let whitelist_code = app.store_code(astro_whitelist());
        let coin_registry_code = app.store_code(astro_coin_registry());
        let factory_code = app.store_code(astro_factory());
        let lper_code = app.store_code(lper_contract());
        let clock_code = app.store_code(clock_contract());
        let holder_code = app.store_code(holder_contract());

        let clock_address = app
            .instantiate_contract(
                clock_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.clock_instantiate,
                &[],
                "clock",
                None,
            )
            .unwrap();

        let holder_address = app
            .instantiate_contract(
                holder_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.holder_instantiate,
                &[],
                "holder",
                Some(CREATOR_ADDR.to_string()),
            )
            .unwrap();
        println!("holder addr: {holder_address:?}");
        self.lp_instantiate.clock_address = clock_address.to_string();
        self.lp_instantiate.holder_address = holder_address.to_string();
        self.factory_instantiate.token_code_id = token_code;
        self.stablepair_instantiate.token_code_id = token_code;
        self.factory_instantiate.whitelist_code_id = whitelist_code;
        self.factory_instantiate.pair_configs[0].code_id = stablepair_code;

        let whitelist_addr = app
            .instantiate_contract(
                whitelist_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.whitelist_instantiate,
                &[],
                "whitelist",
                None,
            )
            .unwrap();

        app.update_block(|b: &mut cosmwasm_std::BlockInfo| b.height += 5);

        let coin_registry_addr = app
            .instantiate_contract(
                coin_registry_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.registry_instantiate,
                &[],
                "native coin registry",
                None,
            )
            .unwrap();
        app.update_block(|b| b.height += 5);

        // add coins to registry
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            coin_registry_addr.clone(),
            &astroport::native_coin_registry::ExecuteMsg::Add {
                native_coins: vec![
                    (ST_ATOM_DENOM.to_string(), 6),
                    (NATIVE_ATOM_DENOM.to_string(), 6),
                ],
            },
            &[],
        )
        .unwrap();
        app.update_block(|b| b.height += 5);

        self.factory_instantiate.coin_registry_address = coin_registry_addr.to_string();

        let factory_addr = app
            .instantiate_contract(
                factory_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.factory_instantiate,
                &[],
                "factory",
                None,
            )
            .unwrap();
        app.update_block(|b| b.height += 5);

        let init_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type: PairType::Stable {},
            asset_infos: vec![
                AssetInfo::NativeToken {
                    denom: ST_ATOM_DENOM.to_string(),
                },
                AssetInfo::NativeToken {
                    denom: NATIVE_ATOM_DENOM.to_string(),
                },
            ],
            init_params: Some(
                to_binary(&StablePoolParams {
                    owner: Some(CREATOR_ADDR.to_string()),
                    amp: 10,
                })
                .unwrap(),
            ),
        };
        app.update_block(|b| b.height += 5);
        println!("init pair msg: {init_pair_msg:?}");
        let pair_msg = app
            .execute_contract(
                Addr::unchecked(CREATOR_ADDR),
                factory_addr.clone(),
                &init_pair_msg,
                &[],
            )
            .unwrap();
        app.update_block(|b| b.height += 5);

        let pair_info: PairInfo = app
            .wrap()
            .query_wasm_smart(
                &factory_addr,
                &astroport::factory::QueryMsg::Pair {
                    asset_infos: vec![
                        AssetInfo::NativeToken {
                            denom: ST_ATOM_DENOM.to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: NATIVE_ATOM_DENOM.to_string(),
                        },
                    ],
                },
            )
            .unwrap();

        self.stablepair_instantiate.factory_addr = factory_addr.to_string();
        app.update_block(|b| b.height += 5);

        let stable_pair_addr = app
            .instantiate_contract(
                stablepair_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.stablepair_instantiate,
                &[],
                "stableswap",
                None,
            )
            .unwrap();

        println!("stablepair : {stable_pair_addr:?}");
        app.update_block(|b| b.height += 5);

        self.lp_instantiate.pool_address = stable_pair_addr.to_string();

        let lper_address = app
            .instantiate_contract(
                lper_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.lp_instantiate,
                &[],
                "lper contract",
                None,
            )
            .unwrap();
        app.update_block(|b| b.height += 5);

        Suite {
            app,
            admin: Addr::unchecked(CREATOR_ADDR),
            lp_token: pair_info.liquidity_token,
            token: token_code,
            whitelist: (whitelist_code, whitelist_addr.to_string()),
            factory: (factory_code, factory_addr.to_string()),
            stable_pair: (stablepair_code, stable_pair_addr.to_string()),
            coin_registry: (coin_registry_code, coin_registry_addr.to_string()),
            liquid_pooler: (lper_code, lper_address.to_string()),
            clock_addr: clock_address.to_string(),
            holder_addr: holder_address.to_string(),
        }
    }
}

// queries
#[allow(unused)]
impl Suite {
    pub fn query_clock_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.liquid_pooler.1, &QueryMsg::ClockAddress {})
            .unwrap()
    }

    pub fn query_lp_position(&self) -> String {
        let lp_config: LpConfig = self.app
            .wrap()
            .query_wasm_smart(&self.liquid_pooler.1, &QueryMsg::LpConfig {})
            .unwrap();
        lp_config.pool_address.to_string()
    }

    pub fn query_contract_state(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.liquid_pooler.1, &QueryMsg::ContractState {})
            .unwrap()
    }

    pub fn query_holder_address(&self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(&self.liquid_pooler.1, &QueryMsg::HolderAddress {})
            .unwrap()
    }

    pub fn query_assets(&self) -> Vec<Asset> {
        self.app
            .wrap()
            .query_wasm_smart(&self.liquid_pooler.1, &QueryMsg::Assets {})
            .unwrap()
    }

    pub fn query_addr_balances(&self, addr: Addr) -> Vec<Coin> {
        self.app.wrap().query_all_balances(addr).unwrap()
    }

    pub fn query_pool_info(&self) -> PoolResponse {
        self.app
            .wrap()
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.stable_pair.clone().1,
                msg: to_binary(&astroport::pair::QueryMsg::Pool {}).unwrap(),
            }))
            .unwrap()
    }

    pub fn query_pool_share(&self) -> Vec<Asset> {
        self.app
            .wrap()
            .query_wasm_smart(
                Addr::unchecked(self.stable_pair.clone().1),
                &astroport::pair::QueryMsg::Share {
                    amount: Uint128::one(),
                },
            )
            .unwrap()
    }

    pub fn query_simulation(&self, addr: String) -> SimulationResponse {
        let query = astroport::pair::QueryMsg::Simulation {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: NATIVE_ATOM_DENOM.to_string(),
                },
                amount: Uint128::one(),
            },
            // ask_asset_info: None,
            ask_asset_info: Some(AssetInfo::NativeToken {
                denom: ST_ATOM_DENOM.to_string(),
            }),
        };
        println!("\nquerying simulation: {query:?}\n");

        self.app.wrap().query_wasm_smart(addr, &query).unwrap()
    }

    pub fn query_contract_config(&self, addr: String) -> String {
        let bytes = self
            .app
            .wrap()
            .query_wasm_raw(addr, b"config")
            .transpose()
            .unwrap()
            .unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(v) => v.to_string(),
            Err(e) => panic!("Invalid UTF-8 sequence: {e}"),
        }
    }

    pub fn query_cw20_bal(&self, token: String, addr: String) -> cw20::BalanceResponse {
        self.app
            .wrap()
            .query_wasm_smart(token, &cw20::Cw20QueryMsg::Balance { address: addr })
            .unwrap()
    }

    pub fn query_liquidity_token_addr(&self) -> astroport::asset::PairInfo {
        self.app
            .wrap()
            .query_wasm_smart(
                self.stable_pair.1.to_string(),
                &astroport::pair::QueryMsg::Pair {},
            )
            .unwrap()
    }
}

// assertion helpers
impl Suite {}

impl Suite {
    // tick LPer
    pub fn tick(&mut self) -> AppResponse {
        self.app
            .execute_contract(
                Addr::unchecked(self.clock_addr.to_string()),
                Addr::unchecked(self.liquid_pooler.1.to_string()),
                &crate::msg::ExecuteMsg::Tick {},
                &[],
            )
            .unwrap()
    }

    // mint coins
    pub fn mint_coins_to_addr(&mut self, address: String, denom: String, amount: Uint128) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: address,
                amount: vec![Coin { amount, denom }],
            }))
            .unwrap();
    }

    // pass time
    pub fn pass_blocks(&mut self, num: u64) {
        self.app.update_block(|b| b.height += num)
    }

    // withdraw liquidity from pool
    #[allow(unused)]
    pub fn withdraw_liquidity(
        &mut self,
        sender: Addr,
        amount: u128,
        assets: Vec<Asset>,
    ) -> AppResponse {
        self.app
            .execute_contract(
                sender,
                Addr::unchecked("contract6".to_string()),
                &Cw20ExecuteMsg::Send {
                    contract: self.stable_pair.1.to_string(),
                    amount: Uint128::from(amount),
                    msg: to_binary(&Cw20HookMsg::WithdrawLiquidity { assets }).unwrap(),
                },
                &[],
            )
            .unwrap()
    }

    pub fn provide_manual_liquidity(
        &mut self,
        from: String,
        st_atom_amount: Uint128,
        native_atom_amount: Uint128,
    ) -> AppResponse {
        let _stable_pair_addr = self.stable_pair.1.to_string();

        let balances = vec![
            Coin {
                denom: ST_ATOM_DENOM.to_string(),
                amount: st_atom_amount,
            },
            Coin {
                denom: NATIVE_ATOM_DENOM.to_string(),
                amount: native_atom_amount,
            },
        ];

        let assets = vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ST_ATOM_DENOM.to_string(),
                },
                amount: st_atom_amount,
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: NATIVE_ATOM_DENOM.to_string(),
                },
                amount: native_atom_amount,
            },
        ];

        self.mint_coins_to_addr(
            from.clone(),
            NATIVE_ATOM_DENOM.to_string(),
            native_atom_amount,
        );
        self.mint_coins_to_addr(from.clone(), ST_ATOM_DENOM.to_string(), st_atom_amount);

        let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance: None,
            auto_stake: Some(false),
            receiver: Some(from.clone()),
        };

        self.pass_blocks(10);

        self.app
            .execute_contract(
                Addr::unchecked(from),
                Addr::unchecked(self.stable_pair.1.to_string()),
                &provide_liquidity_msg,
                &balances,
            )
            .unwrap()
    }

    pub fn holder_withdraw(&mut self) {
        self.app
            .execute_contract(
                Addr::unchecked(CREATOR_ADDR),
                Addr::unchecked(self.holder_addr.to_string()),
                &covenant_holder::msg::ExecuteMsg::WithdrawLiquidity {},
                &[],
            )
            .unwrap();
    }
}
