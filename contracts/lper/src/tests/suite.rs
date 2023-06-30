use std::marker::PhantomData;

use astroport::{asset::{Asset, AssetInfo}, factory::PairConfig, pair::StablePoolParams};
use cosmwasm_std::{Addr, Uint128, testing::{MockStorage, MockApi, MockQuerier}, OwnedDeps, Decimal, Empty, to_binary};
use cw_multi_test::{App, Executor, Contract, ContractWrapper};

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
    pub lper_address: Addr,
    pub lper_code: u64,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: InstantiateMsg {
                clock_address: "default-clock".to_string(),
                lp_position: LPInfo {
                    addr: "lp-addr".to_string(),
                },
                holder_address: "hodler".to_string(),
                slippage_tolerance: Some(Decimal::percent(1)),
                autostake: None,
                assets: vec![
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "uatom".to_string() },
                        amount: Uint128::new(10),
                    },
                    Asset { 
                        info: AssetInfo::NativeToken { denom: "stuatom".to_string() },
                        amount: Uint128::new(10),
                    },                ],
            },
        }
    }
}

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = App::default();

        let token_code = app.store_code(astro_token());
        let stablepair_code = app.store_code(astro_pair_stable());
        let whitelist_code = app.store_code(astro_whitelist());
        let coin_registry_code = app.store_code(astro_coin_registry());
        let factory_code = app.store_code(astro_factory());
        let lper_code = app.store_code(lper_contract());

        let token_addr = app.instantiate_contract(
            token_code,
            Addr::unchecked(CREATOR_ADDR),
            &TokenInstantiateMsg {
                name: "nativetoken".to_string(),
                symbol: "ntk".to_string(),
                decimals: 5,
                initial_balances: vec![],
                mint: None,
                marketing: None,
            },
            &[],
            "astro token",
            None,
        ).unwrap();

        let whitelist_addr = app.instantiate_contract(
            whitelist_code,
            Addr::unchecked(CREATOR_ADDR),
            &WhitelistInstantiateMsg {
                admins: vec![CREATOR_ADDR.to_string()],
                mutable: false,
            },
            &[],
            "whitelist",
            None,
        ).unwrap();
        
        let coin_registry_addr = app.instantiate_contract(
            coin_registry_code,
            Addr::unchecked(CREATOR_ADDR),
            &NativeCoinRegistryInstantiateMsg {
                owner: CREATOR_ADDR.to_string(),
            },
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

        let factory_addr = app.instantiate_contract(
            factory_code,
            Addr::unchecked(CREATOR_ADDR),
            &FactoryInstantiateMsg {
                pair_configs: vec![
                    PairConfig { 
                        code_id: stablepair_code, 
                        pair_type: astroport::factory::PairType::Stable {}, 
                        total_fee_bps: 0, 
                        maker_fee_bps: 0, 
                        is_disabled: false, 
                        is_generator_disabled: true, 
                    },
                ],
                token_code_id: token_code,
                fee_address: None,
                generator_address: None,
                owner: CREATOR_ADDR.to_string(),
                whitelist_code_id: whitelist_code,
                coin_registry_address: coin_registry_addr.to_string(),
            },
            &[],
            "factory",
            None,
        ).unwrap();

        let stableswap_addr = app.instantiate_contract(
            stablepair_code,
            Addr::unchecked(CREATOR_ADDR),
            &PairInstantiateMsg {
                asset_infos: vec![
                    astroport::asset::AssetInfo::NativeToken {
                        denom: ST_ATOM_DENOM.to_string() 
                    },
                    astroport::asset::AssetInfo::NativeToken { 
                        denom: NATIVE_ATOM_DENOM.to_string()
                    },
                ],
                token_code_id: token_code,
                factory_addr: factory_addr.to_string(),
                init_params: Some(to_binary(&StablePoolParams {
                    amp: 9001,
                    owner: Some(CREATOR_ADDR.to_string()),
                }).unwrap()),
            },
            &[],
            "stableswap",
            None,
        ).unwrap();


        let lper_address = app
            .instantiate_contract(
                lper_code,
                Addr::unchecked(CREATOR_ADDR),
                &self.instantiate,
                &[],
                "lper contract",
                Some(CREATOR_ADDR.to_string()),
            )
            .unwrap();

        Suite {
            app,
            admin: Addr::unchecked(CREATOR_ADDR),
            lper_address,
            lper_code,
        }
    }
}

// queries
impl Suite {

}

// assertion helpers
impl Suite {

}