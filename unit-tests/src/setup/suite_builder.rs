use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{
    coin, coins, instantiate2_address, to_json_binary, Addr, Api, CodeInfoResponse, Coin,
};
use cw_multi_test::{
    addons::{MockAddressGenerator, MockApiBech32},
    BasicAppBuilder, Executor, WasmKeeper,
};

use sha2::{Digest, Sha256};

use super::{
    astro_contracts::{
        astro_coin_registry_contract, astro_factory_contract, astro_pair_stable_contract,
        astro_pair_xyk_contract, astro_token_contract, astro_whitelist_contract,
    },
    contracts::{
        astroport_pooler_contract, clock_contract, ibc_forwarder_contract,
        interchain_router_contract, native_router_contract, native_splitter_contract,
        osmo_lp_outpost_contract, remote_splitter_contract, single_party_covenant_contract,
        single_party_holder_contract, stride_lser_contract, swap_covenant_contract,
        swap_holder_contract, two_party_holder_contract,
    },
    custom_module::{NeutronKeeper, CHAIN_PREFIX},
    instantiates::osmo_lp_outpost,
    CustomApp, ADMIN, ALL_DENOMS, DENOM_NTRN, FAUCET, HUB_OSMO_CHANNEL, HUB_STRIDE_CHANNEL,
    NTRN_HUB_CHANNEL, NTRN_OSMO_CHANNEL, NTRN_STRIDE_CHANNEL,
};

pub struct SuiteBuilder {
    pub faucet: Addr,
    pub admin: Addr,

    pub app: CustomApp,

    pub addr_counter: u64,

    // Covenant contracts code ids
    pub swap_covenant_code_id: u64,
    pub single_party_covenant_code_id: u64,

    // Modules code ids
    pub clock_code_id: u64,
    pub swap_holder_code_id: u64,
    pub single_party_holder_code_id: u64,
    pub ibc_forwarder_code_id: u64,
    pub native_router_code_id: u64,
    pub interchain_router_code_id: u64,
    pub remote_splitter_code_id: u64,
    pub native_splitter_code_id: u64,
    pub astro_pooler_code_id: u64,
    pub stride_staker_code_id: u64,
    pub two_party_holder_code_id: u64,
    pub osmo_lp_outpost_code_id: u64,

    // astro contracts
    pub astro_token_code_id: u64,
    pub astro_whitelist_code_id: u64,
    pub astro_factory_code_id: u64,
    pub astro_pair_stable_code_id: u64,
    pub astro_pair_xyk_code_id: u64,
    pub astro_coin_registry_code_id: u64,
}
impl Default for SuiteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SuiteBuilder {
    pub fn new() -> Self {
        let mut app = BasicAppBuilder::new_custom()
            .with_custom(NeutronKeeper::new(CHAIN_PREFIX))
            .with_api(MockApiBech32::new(CHAIN_PREFIX))
            .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
            .build(|r, _, s| {
                let balances: Vec<Coin> = ALL_DENOMS
                    .iter()
                    .map(|d| coin(1_000_000_000_000_000_000_000_000_u128, d.to_string()))
                    .collect();

                r.bank
                    .init_balance(
                        s,
                        &MockApiBech32::new(CHAIN_PREFIX).addr_make(FAUCET),
                        balances,
                    )
                    .unwrap();

                r.custom
                    .add_local_channel(s, NTRN_HUB_CHANNEL.0, NTRN_HUB_CHANNEL.1)
                    .unwrap();
                r.custom
                    .add_local_channel(s, NTRN_OSMO_CHANNEL.0, NTRN_OSMO_CHANNEL.1)
                    .unwrap();
                r.custom
                    .add_local_channel(s, NTRN_STRIDE_CHANNEL.0, NTRN_STRIDE_CHANNEL.1)
                    .unwrap();

                r.custom
                    .add_remote_channel(s, HUB_OSMO_CHANNEL.0, HUB_OSMO_CHANNEL.1)
                    .unwrap();

                r.custom
                    .add_remote_channel(s, HUB_STRIDE_CHANNEL.0, HUB_STRIDE_CHANNEL.1)
                    .unwrap();
            });

        let swap_covenant_code_id = app.store_code(swap_covenant_contract());
        let single_party_covenant_code_id = app.store_code(single_party_covenant_contract());

        let clock_code_id = app.store_code(clock_contract());
        let swap_holder_code_id = app.store_code(swap_holder_contract());
        let single_party_holder_code_id = app.store_code(single_party_holder_contract());
        let remote_splitter_code_id = app.store_code(remote_splitter_contract());
        let native_splitter_code_id = app.store_code(native_splitter_contract());
        let interchain_router_code_id = app.store_code(interchain_router_contract());
        let native_router_code_id = app.store_code(native_router_contract());
        let ibc_forwarder_code_id = app.store_code(ibc_forwarder_contract());
        let astro_pooler_code_id = app.store_code(astroport_pooler_contract());
        let stride_staker_code_id = app.store_code(stride_lser_contract());
        let two_party_holder_code_id = app.store_code(two_party_holder_contract());
        let osmo_lp_outpost_code_id = app.store_code(osmo_lp_outpost_contract());

        let astro_token_code_id = app.store_code(astro_token_contract());
        let astro_whitelist_code_id = app.store_code(astro_whitelist_contract());
        let astro_factory_code_id = app.store_code(astro_factory_contract());
        let astro_pair_stable_code_id = app.store_code(astro_pair_stable_contract());
        let astro_pair_xyk_code_id = app.store_code(astro_pair_xyk_contract());
        let astro_coin_registry_code_id = app.store_code(astro_coin_registry_contract());

        Self {
            faucet: app.api().addr_make(FAUCET),
            admin: app.api().addr_make(ADMIN),

            app,
            addr_counter: 0,

            swap_covenant_code_id,
            single_party_covenant_code_id,

            clock_code_id,
            swap_holder_code_id,
            single_party_holder_code_id,
            ibc_forwarder_code_id,
            native_router_code_id,
            interchain_router_code_id,
            remote_splitter_code_id,
            native_splitter_code_id,
            astro_pooler_code_id,
            stride_staker_code_id,
            two_party_holder_code_id,
            osmo_lp_outpost_code_id,

            astro_token_code_id,
            astro_whitelist_code_id,
            astro_factory_code_id,
            astro_pair_stable_code_id,
            astro_pair_xyk_code_id,
            astro_coin_registry_code_id,
        }
    }

    // Init pool and return the addr
    pub fn init_astro_pool(
        &mut self,
        pair_type: astroport::factory::PairType,
        coin_a: Coin,
        coin_b: Coin,
    ) -> (Addr, Addr) {
        let registery_init = astroport::native_coin_registry::InstantiateMsg {
            owner: self.admin.to_string(),
        };
        let coin_registry_addr = self
            .app
            .instantiate_contract(
                self.astro_coin_registry_code_id,
                self.admin.clone(),
                &registery_init,
                &[],
                "native coin registry",
                None,
            )
            .unwrap();
        self.app.update_block(|b| b.height += 5);

        // Add coins to the registery
        self.app
            .execute_contract(
                self.admin.clone(),
                coin_registry_addr.clone(),
                &astroport::native_coin_registry::ExecuteMsg::Add {
                    native_coins: vec![(coin_a.denom.clone(), 6), (coin_b.denom.clone(), 6)],
                },
                &[],
            )
            .unwrap();
        self.app.update_block(|b| b.height += 5);

        //init factory
        let factory_init = astroport::factory::InstantiateMsg {
            pair_configs: vec![
                astroport::factory::PairConfig {
                    code_id: self.astro_pair_stable_code_id,
                    pair_type: astroport::factory::PairType::Stable {},
                    total_fee_bps: 0,
                    maker_fee_bps: 0,
                    is_disabled: false,
                    is_generator_disabled: true,
                },
                astroport::factory::PairConfig {
                    code_id: self.astro_pair_xyk_code_id,
                    pair_type: astroport::factory::PairType::Xyk {},
                    total_fee_bps: 0,
                    maker_fee_bps: 0,
                    is_disabled: false,
                    is_generator_disabled: true,
                },
            ],
            token_code_id: self.astro_token_code_id,
            fee_address: None,
            generator_address: None,
            owner: self.admin.to_string(),
            whitelist_code_id: self.astro_whitelist_code_id,
            coin_registry_address: coin_registry_addr.to_string(),
        };
        let factory_addr = self
            .app
            .instantiate_contract(
                self.astro_factory_code_id,
                self.admin.clone(),
                &factory_init,
                &[],
                "factory",
                None,
            )
            .unwrap();
        self.app.update_block(|b| b.height += 5);

        let asset_infos = vec![
            astroport::asset::AssetInfo::NativeToken {
                denom: coin_a.denom.clone(),
            },
            astroport::asset::AssetInfo::NativeToken {
                denom: coin_b.denom.clone(),
            },
        ];

        let init_params = match &pair_type {
            astroport::factory::PairType::Stable {} => {
                to_json_binary(&astroport::pair::StablePoolParams {
                    amp: 1,
                    owner: Some(self.admin.to_string()),
                })
                .unwrap()
            }
            astroport::factory::PairType::Xyk {} => {
                to_json_binary(&astroport::pair::XYKPoolParams {
                    track_asset_balances: None,
                })
                .unwrap()
            }
            astroport::factory::PairType::Custom(_) => {
                panic!("suite-builder: custom pair type is not supported")
            }
        };

        let init_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type: pair_type.clone(),
            asset_infos: asset_infos.clone(),
            init_params: Some(init_params),
        };
        self.app
            .execute_contract(
                self.admin.clone(),
                factory_addr.clone(),
                &init_pair_msg,
                &[],
            )
            .unwrap();
        self.app.update_block(|b| b.height += 5);
        let pool_info = self
            .app
            .wrap()
            .query_wasm_smart::<astroport::asset::PairInfo>(
                factory_addr.clone(),
                &astroport::factory::QueryMsg::Pair { asset_infos },
            )
            .unwrap();

        // provide liquidity to the pool
        let balances = vec![coin_a.clone(), coin_b.clone()];

        let assets = vec![
            astroport::asset::Asset {
                info: astroport::asset::AssetInfo::NativeToken {
                    denom: coin_a.denom.to_string(),
                },
                amount: coin_a.amount,
            },
            astroport::asset::Asset {
                info: astroport::asset::AssetInfo::NativeToken {
                    denom: coin_b.denom,
                },
                amount: coin_b.amount,
            },
        ];

        let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance: None,
            auto_stake: Some(false),
            receiver: Some(self.faucet.to_string()),
        };

        self.app
            .execute_contract(
                self.faucet.clone(),
                pool_info.contract_addr.clone(),
                &provide_liquidity_msg,
                &balances,
            )
            .unwrap();

        (pool_info.contract_addr, pool_info.liquidity_token)
    }
    /// Add IBC channels for the neutron module
    pub fn add_channels(
        &mut self,
        local: Vec<(&str, &str)>,
        remote: Vec<(&str, &str)>,
    ) -> &mut Self {
        self.app.init_modules(|r, _, s| {
            local
                .iter()
                .for_each(|(source, other)| r.custom.add_local_channel(s, source, other).unwrap());
            remote
                .iter()
                .for_each(|(some, other)| r.custom.add_remote_channel(s, some, other).unwrap());
        });
        self
    }

    pub fn fund_with_ntrn(&mut self, addr: &Addr, amount: u128) {
        self.app
            .send_tokens(
                self.faucet.clone(),
                addr.clone(),
                &coins(amount, DENOM_NTRN),
            )
            .unwrap();
    }

    // Consume the builder and return the app
    pub fn build(self) -> CustomApp {
        self.app
    }
}

impl SuiteBuilder {
    pub fn get_random_addr(&mut self) -> Addr {
        self.addr_counter += 1;
        self.app
            .api()
            .addr_make(format!("random_addr-{}", self.addr_counter).as_str())
    }

    pub fn get_contract_addr(&mut self, code_id: u64, salt: &str) -> Addr {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        let salt = hasher.finalize().to_vec();

        let canonical_creator = self
            .app
            .api()
            .addr_canonicalize(self.app.api().addr_make(ADMIN).as_str())
            .unwrap();
        let CodeInfoResponse { checksum, .. } =
            self.app.wrap().query_wasm_code_info(code_id).unwrap();
        let canonical_addr = instantiate2_address(&checksum, &canonical_creator, &salt).unwrap();
        self.app.api().addr_humanize(&canonical_addr).unwrap()
    }

    pub fn contract_init<M: Serialize>(
        &mut self,
        code_id: u64,
        label: String,
        init_msg: &M,
        funds: &[Coin],
    ) -> Addr {
        self.app
            .instantiate_contract(
                code_id,
                self.app.api().addr_make(ADMIN),
                &init_msg,
                funds,
                label,
                Some(ADMIN.to_string()),
            )
            .unwrap()
    }

    pub fn contract_init2<M: Serialize>(
        &mut self,
        code_id: u64,
        salt: &str,
        init_msg: &M,
        funds: &[Coin],
    ) -> Addr {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        let hashed_salt = hasher.finalize().to_vec();

        self.app
            .instantiate2_contract(
                code_id,
                self.app.api().addr_make(ADMIN),
                &init_msg,
                funds,
                salt.to_string(),
                Some(ADMIN.to_string()),
                hashed_salt,
            )
            .unwrap()
    }
}
