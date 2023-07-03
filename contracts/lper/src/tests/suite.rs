use std::marker::PhantomData;

use astroport::{asset::{Asset, AssetInfo, PairInfo}, factory::{PairConfig, PairType}, pair::{StablePoolParams, Cw20HookMsg, PoolResponse, ConfigResponse, SimulationResponse}};
use astroport_pair_stable::error::ContractError;
use cosmwasm_std::{Addr, Uint128, testing::{MockStorage, MockApi, MockQuerier}, OwnedDeps, Decimal, Empty, to_binary, Coin, QueryRequest, WasmQuery, Response, StdResult, Binary};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{App, Executor, Contract, ContractWrapper, SudoMsg, BankSudo, AppResponse};

use crate::{msg::{InstantiateMsg, QueryMsg, LPInfo}};
use astroport::token::InstantiateMsg as TokenInstantiateMsg;
use astroport::factory::InstantiateMsg as FactoryInstantiateMsg;
use cw1_whitelist::msg::InstantiateMsg as WhitelistInstantiateMsg;
use astroport::native_coin_registry::InstantiateMsg as NativeCoinRegistryInstantiateMsg;
use astroport::pair::InstantiateMsg as PairInstantiateMsg;

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
        .with_migrate(astroport_token::contract::migrate)
    )
}

fn astro_whitelist() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            astroport_whitelist::contract::instantiate,
            astroport_whitelist::contract::instantiate,
            astroport_whitelist::contract::query,
        )
    )
}

fn astro_factory() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
        astroport_factory::contract::execute,
        astroport_factory::contract::instantiate,
        astroport_factory::contract::query
        )
        .with_migrate(astroport_factory::contract::migrate)
        .with_reply(astroport_factory::contract::reply)
    )
}

fn astro_pair_stable() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
        astroport_pair_stable::contract::execute,
        astroport_pair_stable::contract::instantiate,
        astroport_pair_stable::contract::query
        ) 
        .with_reply(astroport_pair_stable::contract::reply)
        .with_migrate(astroport_pair_stable::contract::migrate)
    ) 
}


fn astro_coin_registry() -> Box<dyn Contract<Empty>> {
    let registry_contract = ContractWrapper::new(
        astroport_native_coin_registry::contract::execute,
        astroport_native_coin_registry::contract::instantiate,
        astroport_native_coin_registry::contract::query
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
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);

    Box::new(lp_contract)
}
#[allow(unused)]
pub(crate) struct Suite {
    pub app: App,
    pub admin: Addr,
    pub lp_token: Addr,
    // (token_code, contract_address)
    pub token: (u64, String),
    pub whitelist: (u64, String),
    pub factory: (u64, String),
    pub stable_pair: (u64, String),
    pub coin_registry: (u64, String),
    pub liquid_pooler: (u64, String),
}

pub(crate) struct SuiteBuilder {
    pub lp_instantiate: InstantiateMsg,
    pub token_instantiate: TokenInstantiateMsg,
    pub whitelist_instantiate: WhitelistInstantiateMsg,
    pub factory_instantiate: FactoryInstantiateMsg,
    pub stablepair_instantiate: PairInstantiateMsg,
    pub registry_instantiate: NativeCoinRegistryInstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            lp_instantiate: InstantiateMsg {
                clock_address: "default-clock".to_string(),
                lp_position: LPInfo {
                    addr: "lp-addr".to_string(),
                },
                holder_address: "hodler".to_string(),
                slippage_tolerance: Some(Decimal::zero()),
                autostake: Some(false),
                assets: vec![
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "uatom".to_string() },
                        amount: Uint128::new(1000),
                    },
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "stuatom".to_string() },
                        amount: Uint128::new(1000),
                    },                
                ],
            },
            token_instantiate: TokenInstantiateMsg {
                name: "nativetoken".to_string(),
                symbol: "ntk".to_string(),
                decimals: 5,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            whitelist_instantiate: WhitelistInstantiateMsg {
                admins: vec![CREATOR_ADDR.to_string()],
                mutable: false,
            },
            factory_instantiate: FactoryInstantiateMsg {
                pair_configs: vec![
                    PairConfig { 
                        code_id: u64::MAX, 
                        pair_type: astroport::factory::PairType::Stable {}, 
                        total_fee_bps: 0, 
                        maker_fee_bps: 0, 
                        is_disabled: false, 
                        is_generator_disabled: true, 
                    },
                ],
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
                        denom: ST_ATOM_DENOM.to_string() 
                    },
                    astroport::asset::AssetInfo::NativeToken { 
                        denom: NATIVE_ATOM_DENOM.to_string()
                    },
                ],
                token_code_id: u64::MAX,
                factory_addr: "TODO".to_string(),
                init_params: Some(to_binary(&StablePoolParams {
                    amp: 1,
                    owner: Some(CREATOR_ADDR.to_string()),
                }).unwrap()),
            },
            registry_instantiate: NativeCoinRegistryInstantiateMsg {
                owner: CREATOR_ADDR.to_string(),
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

    fn with_assets(mut self, assets: Vec<Asset>) -> Self {
        self.lp_instantiate.assets = assets;
        self
    }

    fn with_token_instantiate_msg(mut self, msg: TokenInstantiateMsg) -> Self {
        self.token_instantiate = msg;
        self
    }

    pub fn build(mut self) -> Suite {
        let mut app = App::default();
 
        let token_code = app.store_code(astro_token());
        let stablepair_code = app.store_code(astro_pair_stable());
        let whitelist_code = app.store_code(astro_whitelist());
        let coin_registry_code = app.store_code(astro_coin_registry());
        let factory_code = app.store_code(astro_factory());
        let lper_code = app.store_code(lper_contract());

        self.factory_instantiate.token_code_id = token_code;
        self.stablepair_instantiate.token_code_id = token_code;
        self.factory_instantiate.whitelist_code_id = whitelist_code;
        self.factory_instantiate.pair_configs[0].code_id = stablepair_code;

        // println!("token instantiate: {:?}\n\n", self.token_instantiate);
        // let token_addr = app.instantiate_contract(
        //     token_code,
        //     Addr::unchecked(CREATOR_ADDR),
        //     &self.token_instantiate,
        //     &[],
        //     "astro token",
        //     None,
        // ).unwrap();
        let token_addr = "random".to_string();

        // println!("whitelist instantiate: {:?}\n\n", self.whitelist_instantiate);
        let whitelist_addr = app.instantiate_contract(
            whitelist_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.whitelist_instantiate,
            &[],
            "whitelist",
            None,
        ).unwrap();
        

        // println!("registry instantiate: {:?}\n\n", self.registry_instantiate);
        let coin_registry_addr = app.instantiate_contract(
            coin_registry_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.registry_instantiate,
            &[],
            "native coin registry",
            None
        ).unwrap();
        // add coins to registry
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            coin_registry_addr.clone(),
            &astroport::native_coin_registry::ExecuteMsg::Add { 
                native_coins: vec![
                    (ST_ATOM_DENOM.to_string(), 10),
                    (NATIVE_ATOM_DENOM.to_string(), 10),
                ]
            },
            &[],
        ).unwrap();

        self.factory_instantiate.coin_registry_address = coin_registry_addr.to_string();

        // println!("factory instantiate: {:?}\n\n", self.factory_instantiate);
        let factory_addr = app.instantiate_contract(
            factory_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.factory_instantiate,
            &[],
            "factory",
            None,
        ).unwrap();

        let init_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type: PairType::Stable {},
            asset_infos: vec![
                AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() },
                AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() },
            ],
            init_params: Some(to_binary(&StablePoolParams { 
                owner: Some(CREATOR_ADDR.to_string()),
                amp: 9001,
             }).unwrap()),
        };
        let pair_msg = app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            factory_addr.clone(),
            &init_pair_msg,
            &[]
        ).unwrap();

        let pair_info: PairInfo = app.wrap().query_wasm_smart(
            &factory_addr,
            &astroport::factory::QueryMsg::Pair { asset_infos: vec![
                    AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() },
                    AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() },
                ] 
            },
        ).unwrap();
        // println!("\n pair info: {:?}", pair_info);

        self.stablepair_instantiate.factory_addr = factory_addr.to_string();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: CREATOR_ADDR.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(1000),
                denom: ST_ATOM_DENOM.to_string(),
            }],
        }))
        .unwrap();
        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: CREATOR_ADDR.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(1000),
                denom: NATIVE_ATOM_DENOM.to_string(),
            }],
        }))
        .unwrap();

        // println!("stableswap instantiate: {:?}\n\n", self.stablepair_instantiate);
        let stableswap_address = app.instantiate_contract(
            stablepair_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.stablepair_instantiate,
            &[],
            "stableswap",
            None,
        ).unwrap();

        app.update_block(|b| b.height += 5);

        self.lp_instantiate.lp_position.addr = stableswap_address.to_string();
        // let resp = app.wrap().query_wasm_raw(
        //     stableswap_address.to_string(),
        //     b"config",
        // ).transpose().unwrap().unwrap();
        // let s = match std::str::from_utf8(&resp) {
        //     Ok(v) => v,
        //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        // };
        // println!("\n raw query {:?}\n", s);

        println!("lper instantiate: {:?}\n\n", self.lp_instantiate);
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

        Suite {
            app,
            admin: Addr::unchecked(CREATOR_ADDR),
            lp_token: pair_info.liquidity_token.clone(),
            token: (token_code, token_addr.to_string()),
            whitelist: (whitelist_code, whitelist_addr.to_string()),
            factory: (factory_code, factory_addr.to_string()),
            stable_pair: (stablepair_code, stableswap_address.to_string()),
            coin_registry: (coin_registry_code, coin_registry_addr.to_string()),
            liquid_pooler: (lper_code, lper_address.to_string()),
        }
    }
}

// queries
#[allow(unused)]
impl Suite {
    pub fn query_clock_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.liquid_pooler.1,
                &QueryMsg::ClockAddress {}
            )    
            .unwrap()
    }

    pub fn query_lp_position(&self) -> LPInfo {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.liquid_pooler.1,
                &QueryMsg::LpPosition {}
            )    
            .unwrap()
    }

    pub fn query_contract_state(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.liquid_pooler.1,
                &QueryMsg::ContractState {}
            )    
            .unwrap()
    }

    pub fn query_holder_address(&self) -> String {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.liquid_pooler.1,
                &QueryMsg::HolderAddress {}
            )    
            .unwrap()
    }

    pub fn query_assets(&self) -> Vec<Asset> {
        self.app    
            .wrap()    
            .query_wasm_smart(
                &self.liquid_pooler.1,
                &QueryMsg::Assets {}
            )    
            .unwrap()
    }

    pub fn query_addr_balances(&self, addr: Addr) -> Vec<Coin> {
        self.app.wrap()
            .query_all_balances(addr)
            .unwrap()
    }

    pub fn query_pool_info(&self) -> PoolResponse {
        self.app.wrap().query(
            &QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.stable_pair.clone().1,
                msg: to_binary(&astroport::pair::QueryMsg::Pool {}).unwrap(),
            })
        ).unwrap()
    }

    pub fn query_pool_share(&self) -> Vec<Asset> {
        self.app.wrap().query_wasm_smart(
            Addr::unchecked(self.stable_pair.clone().1),
            &astroport::pair::QueryMsg::Share { amount: Uint128::one() },
        ).unwrap()
    }

    pub fn query_simulation(&self) -> SimulationResponse {
        self.app.wrap().query_wasm_smart(
            self.stable_pair.clone().1,
            &astroport::pair::QueryMsg::Simulation { 
                offer_asset: Asset { 
                    info: AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() },
                    amount: Uint128::one(),
                },
                ask_asset_info: Some(AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() }),
            }
        ).unwrap()
    }
}

// assertion helpers
impl Suite {

}

impl Suite {
    // tick LPer
    pub fn tick(&mut self) -> AppResponse {
        self.app.execute_contract(
            Addr::unchecked(CREATOR_ADDR), 
            Addr::unchecked(self.liquid_pooler.1.to_string()),
            &crate::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap()
    }

    // mint coins
    pub fn mint_coins_to_addr(&mut self, address: String, denom: String, amount: Uint128) {
        self.app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: address.to_string(),
            amount: vec![Coin {
                amount,
                denom,
            }],
        }))
        .unwrap();
    }

    // pass time
    pub fn pass_blocks(&mut self, num: u64) {
        self.app.update_block(|b| b.height += num)
    }

    // withdraw liquidity from pool
    pub fn withdraw_liquidity(&mut self, sender: &Addr, amount: u128, assets: Vec<Asset>) -> AppResponse {
        let msg = Cw20ExecuteMsg::Send {
            contract: self.stable_pair.1.to_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&Cw20HookMsg::WithdrawLiquidity { assets }).unwrap(),
        };

        self.app.execute_contract(
            sender.clone(),
            self.lp_token.clone(),
            &msg,
            &[],
        ).unwrap()
    }

    pub fn provide_manual_liquidity(&mut self) -> AppResponse {
        let balances = vec![
            Coin { 
                denom: ST_ATOM_DENOM.to_string(), 
                amount: Uint128::new(5000),
            },
            Coin { 
                denom: NATIVE_ATOM_DENOM.to_string(), 
                amount: Uint128::new(5000),
            },
        ];

        let assets = vec![
            Asset { 
                info: AssetInfo::NativeToken { denom: ST_ATOM_DENOM.to_string() }, 
                amount: Uint128::new(5000),
            },
            Asset { 
                info: AssetInfo::NativeToken { denom: NATIVE_ATOM_DENOM.to_string() }, 
                amount: Uint128::new(5000),
            },
        ];

        self.mint_coins_to_addr("alice".to_string(), NATIVE_ATOM_DENOM.to_string(), Uint128::new(10000));
        self.mint_coins_to_addr("alice".to_string(), ST_ATOM_DENOM.to_string(), Uint128::new(10000));

        let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance: None,
            auto_stake: Some(false),
            receiver: Some("alice".to_string()),
        };

        self.pass_blocks(10);

        self.app.execute_contract(
            Addr::unchecked("alice".to_string()), 
            Addr::unchecked(self.stable_pair.1.to_string()),
            &provide_liquidity_msg,
            &balances,
        ).unwrap()
    }
}