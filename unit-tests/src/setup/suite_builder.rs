use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{coin, instantiate2_address, Addr, Api, CodeInfoResponse, Coin};
use cw_multi_test::{
    addons::{MockAddressGenerator, MockApiBech32},
    BasicAppBuilder, Executor, WasmKeeper,
};

use sha2::{Digest, Sha256};

use super::{
    contracts::{
        clock_contract, ibc_forwarder_contract, interchain_router_contract, native_router_contract,
        native_splitter_contract, remote_splitter_contract, swap_covenant_contract,
        swap_holder_contract,
    },
    custom_module::{NeutronKeeper, CHAIN_PREFIX},
    CustomApp, ADMIN, ALL_DENOMS, FAUCET, HUB_OSMO_CHANNEL, NTRN_HUB_CHANNEL, NTRN_OSMO_CHANNEL,
};

pub struct SuiteBuilder {
    pub fuacet: Addr,
    pub admin: Addr,

    pub app: CustomApp,

    pub addr_counter: u64,

    pub clock_code_id: u64,
    pub swap_covenant_code_id: u64,
    pub swap_holder_code_id: u64,
    pub ibc_forwarder_code_id: u64,
    pub native_router_code_id: u64,
    pub interchain_router_code_id: u64,
    pub remote_splitter_code_id: u64,
    pub native_splitter_code_id: u64,
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
                    .add_remote_channel(s, HUB_OSMO_CHANNEL.0, HUB_OSMO_CHANNEL.1)
                    .unwrap();
            });

        let clock_code_id = app.store_code(clock_contract());
        let swap_covenant_code_id = app.store_code(swap_covenant_contract());
        let swap_holder_code_id = app.store_code(swap_holder_contract());
        let remote_splitter_code_id = app.store_code(remote_splitter_contract());
        let native_splitter_code_id = app.store_code(native_splitter_contract());
        let interchain_router_code_id = app.store_code(interchain_router_contract());
        let native_router_code_id = app.store_code(native_router_contract());
        let ibc_forwarder_code_id = app.store_code(ibc_forwarder_contract());

        Self {
            fuacet: app.api().addr_make(FAUCET),
            admin: app.api().addr_make(ADMIN),

            app,
            addr_counter: 0,

            clock_code_id,
            swap_covenant_code_id,
            swap_holder_code_id,
            ibc_forwarder_code_id,
            native_router_code_id,
            interchain_router_code_id,
            remote_splitter_code_id,
            native_splitter_code_id,
        }
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

    pub fn contract_init2<M: Serialize>(
        &mut self,
        code_id: u64,
        salt: &str,
        init_msg: &M,
        funds: &[Coin],
    ) {
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
            .unwrap();
    }
}
