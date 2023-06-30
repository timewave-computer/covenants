use std::marker::PhantomData;

use astroport::{asset::{Asset, AssetInfo}, factory::PairConfig, pair::StablePoolParams};
use cosmwasm_std::{Addr, Uint128, testing::{MockStorage, MockApi, MockQuerier}, OwnedDeps, Decimal, Empty, to_binary, Coin};
use cw_multi_test::{App, Executor, Contract, ContractWrapper, SudoMsg, BankSudo};

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
    Box::new(ContractWrapper::new(
        astroport_factory::contract::execute,
        astroport_factory::contract::instantiate,
        astroport_factory::contract::query
    ))
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
    Box::new(ContractWrapper::new(
        astroport_native_coin_registry::contract::execute,
        astroport_native_coin_registry::contract::instantiate,
        astroport_native_coin_registry::contract::query
    ))
}

fn lper_contract() -> Box<dyn Contract<Empty>> {
    let lp_contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );

    Box::new(lp_contract)
}

pub(crate) struct Suite {
    pub app: App,
    pub admin: Addr,
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
                autostake: None,
                assets: vec![
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "uatom".to_string() },
                        amount: Uint128::new(10),
                    },
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "stuatom".to_string() },
                        amount: Uint128::new(10),
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
                    amp: 9001,
                    owner: Some(CREATOR_ADDR.to_string()),
                }).unwrap()),
            },
            registry_instantiate: NativeCoinRegistryInstantiateMsg {
                owner: CREATOR_ADDR.to_string(),
            },
        }
    }
}

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

        let token_addr = app.instantiate_contract(
            token_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.token_instantiate,
            &[],
            "astro token",
            None,
        ).unwrap();

        let whitelist_addr = app.instantiate_contract(
            whitelist_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.whitelist_instantiate,
            &[],
            "whitelist",
            None,
        ).unwrap();
        
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

        let factory_addr = app.instantiate_contract(
            factory_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.factory_instantiate,
            &[],
            "factory",
            None,
        ).unwrap();

        self.stablepair_instantiate.factory_addr = factory_addr.to_string();
        
        let stableswap_address = app.instantiate_contract(
            stablepair_code,
            Addr::unchecked(CREATOR_ADDR),
            &self.stablepair_instantiate,
            &[],
            "stableswap",
            None,
        ).unwrap();

        self.lp_instantiate.lp_position.addr = stableswap_address.to_string();

        let lper_address = app
            .instantiate_contract(
                lper_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.lp_instantiate,
                &[],
                "lper contract",
                Some(CREATOR_ADDR.to_string()),
            )
            .unwrap();

        Suite {
            app,
            admin: Addr::unchecked(CREATOR_ADDR),
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
}

// assertion helpers
impl Suite {

}

impl Suite {
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
}