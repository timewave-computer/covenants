use std::collections::BTreeMap;

use cosmwasm_std::{Addr, Coin, Uint128, Uint64};
use covenant_utils::{neutron::RemoteChainInfo, split::SplitConfig};
use cw_multi_test::{AppResponse, Executor};

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::remote_chain_splitter::RemoteChainSplitterInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, NTRN_HUB_CHANNEL, REMOTE_CHAIN_SPLITTER_SALT,
};

pub struct RemoteChainSplitterBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: RemoteChainSplitterInstantiate,
}

impl Default for RemoteChainSplitterBuilder {
    fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let remote_chain_splitter_addr =
            builder.get_contract_addr(builder.remote_splitter_code_id, REMOTE_CHAIN_SPLITTER_SALT);

        let forwarder_a_addr =
            builder.get_contract_addr(builder.ibc_forwarder_code_id, "forwarder_a");
        let forwarder_b_addr =
            builder.get_contract_addr(builder.ibc_forwarder_code_id, "forwarder_b");
        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![
                remote_chain_splitter_addr.to_string(),
                forwarder_a_addr.to_string(),
                forwarder_b_addr.to_string(),
            ],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let default_forwarder_instantiate_msg = valence_ibc_forwarder::msg::InstantiateMsg {
            privileged_addresses: Some(vec![clock_addr.to_string()]),
            next_contract: clock_addr.to_string(),
            remote_chain_connection_id: "connection-0".to_string(),
            remote_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
            denom: DENOM_ATOM_ON_NTRN.to_string(),
            amount: Uint128::new(100),
            ibc_transfer_timeout: Uint64::new(100),
            ica_timeout: Uint64::new(100),
            fallback_address: None,
        };

        builder.contract_init2(
            builder.ibc_forwarder_code_id,
            "forwarder_a",
            &default_forwarder_instantiate_msg,
            &[],
        );
        builder.contract_init2(
            builder.ibc_forwarder_code_id,
            "forwarder_b",
            &default_forwarder_instantiate_msg,
            &[],
        );

        let remote_chain_splitter_instantiate = RemoteChainSplitterInstantiate::default(
            clock_addr.to_string(),
            forwarder_a_addr.to_string(),
            forwarder_b_addr.to_string(),
        );

        Self {
            builder,
            instantiate_msg: remote_chain_splitter_instantiate,
        }
    }
}

#[allow(dead_code)]
impl RemoteChainSplitterBuilder {
    pub fn with_clock_address(mut self, addr: String) -> Self {
        self.instantiate_msg.with_clock_address(addr);
        self
    }

    pub fn with_remote_chain_connection_id(mut self, id: String) -> Self {
        self.instantiate_msg.with_remote_chain_connection_id(id);
        self
    }

    pub fn with_remote_chain_channel_id(mut self, id: String) -> Self {
        self.instantiate_msg.with_remote_chain_channel_id(id);
        self
    }

    pub fn with_denom(mut self, denom: String) -> Self {
        self.instantiate_msg.with_denom(denom);
        self
    }

    pub fn with_amount(mut self, amount: Uint128) -> Self {
        self.instantiate_msg.with_amount(amount);
        self
    }

    pub fn with_splits(mut self, splits: BTreeMap<String, SplitConfig>) -> Self {
        self.instantiate_msg.with_splits(splits);
        self
    }

    pub fn with_ica_timeout(mut self, ica_timeout: Uint64) -> Self {
        self.instantiate_msg.with_ica_timeout(ica_timeout);
        self
    }

    pub fn with_ibc_transfer_timeout(mut self, ibc_transfer_timeout: Uint64) -> Self {
        self.instantiate_msg
            .with_ibc_transfer_timeout(ibc_transfer_timeout);
        self
    }

    pub fn build(mut self) -> Suite {
        let remote_chain_splitter_address = self.builder.contract_init2(
            self.builder.remote_splitter_code_id,
            REMOTE_CHAIN_SPLITTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let split_config: Vec<(String, SplitConfig)> = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::SplitConfig {},
            )
            .unwrap();
        let config_1 = split_config[0].clone().1.receivers;
        let receivers: Vec<String> = config_1.keys().cloned().collect();

        let splits = BTreeMap::from_iter(split_config);

        let transfer_amount = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::TransferAmount {},
            )
            .unwrap();

        let remote_chain_info = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap();

        Suite {
            splitter: remote_chain_splitter_address,
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr,
            splits,
            transfer_amount,
            remote_chain_info,
            app: self.builder.build(),
            receiver_1: Addr::unchecked(receivers[0].to_string()),
            receiver_2: Addr::unchecked(receivers[1].to_string()),
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub clock_addr: Addr,
    pub splitter: Addr,

    pub splits: BTreeMap<String, SplitConfig>,
    pub transfer_amount: Uint128,
    pub remote_chain_info: RemoteChainInfo,

    pub receiver_1: Addr,
    pub receiver_2: Addr,
}

impl Suite {
    pub fn query_clock_address(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap()
    }

    pub fn query_contract_state(&self) -> valence_remote_chain_splitter::msg::ContractState {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::ContractState {},
            )
            .unwrap()
    }

    pub fn query_remote_chain_info(&self) -> RemoteChainInfo {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap()
    }

    pub fn query_split_config(&self) -> BTreeMap<String, SplitConfig> {
        let split_config: Vec<(String, SplitConfig)> = self
            .app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::SplitConfig {},
            )
            .unwrap();
        BTreeMap::from_iter(split_config)
    }

    pub fn query_transfer_amount(&self) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::TransferAmount {},
            )
            .unwrap()
    }

    pub fn query_deposit_address(&self, addr: Addr) -> Option<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                addr,
                &valence_remote_chain_splitter::msg::QueryMsg::DepositAddress {},
            )
            .unwrap()
    }

    pub fn query_fallback_address(&self) -> Option<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::QueryMsg::FallbackAddress {},
            )
            .unwrap()
    }

    pub fn distribute_fallback(&mut self, coins: Vec<Coin>, funds: Vec<Coin>) -> AppResponse {
        self.app
            .execute_contract(
                self.faucet.clone(),
                self.splitter.clone(),
                &valence_remote_chain_splitter::msg::ExecuteMsg::DistributeFallback { coins },
                &funds,
            )
            .unwrap()
    }

    pub fn query_ica_address(&mut self, addr: Addr) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(
                addr,
                &valence_remote_chain_splitter::msg::QueryMsg::IcaAddress {},
            )
            .unwrap()
    }
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        self.clock_addr.clone()
    }

    fn get_faucet_addr(&mut self) -> Addr {
        self.faucet.clone()
    }
}

impl BaseSuite for Suite {
    fn get_app(&self) -> &CustomApp {
        &self.app
    }
}
