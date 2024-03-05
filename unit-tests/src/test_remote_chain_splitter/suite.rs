use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128, Uint64};
use covenant_utils::{neutron::RemoteChainInfo, split::SplitConfig};
use neutron_sdk::bindings::msg::IbcFee;

use crate::setup::{
    base_suite::BaseSuiteMut, instantiates::remote_chain_splitter::RemoteChainSplitterInstantiate, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NTRN_HUB_CHANNEL, REMOTE_CHAIN_SPLITTER_SALT
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

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![remote_chain_splitter_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_a_controller_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

    
        let remote_chain_splitter_instantiate = RemoteChainSplitterInstantiate::default(
            clock_addr.to_string(),
            party_a_controller_addr.to_string(),
            party_b_controller_addr.to_string(),
        );

        Self {
            builder,
            instantiate_msg: remote_chain_splitter_instantiate,
        }

    }
}

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

    pub fn with_ibc_fee(mut self, ibc_fee: IbcFee) -> Self {
        self.instantiate_msg.with_ibc_fee(ibc_fee);
        self
    }

    pub fn with_ica_timeout(mut self, ica_timeout: Uint64) -> Self {
        self.instantiate_msg.with_ica_timeout(ica_timeout);
        self
    }

    pub fn with_ibc_transfer_timeout(mut self, ibc_transfer_timeout: Uint64) -> Self {
        self.instantiate_msg.with_ibc_transfer_timeout(ibc_transfer_timeout);
        self
    }

    pub fn build(mut self) -> Suite {
        let remote_chain_splitter_address = self.builder.contract_init2(
            self.builder.remote_splitter_code_id,
            REMOTE_CHAIN_SPLITTER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let clock_addr = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let split_config: Vec<(String, SplitConfig)> = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::SplitConfig {},
            )
            .unwrap();
        let splits = BTreeMap::from_iter(split_config);

        let transfer_amount = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::TransferAmount {},
            )
            .unwrap();

        let remote_chain_info = self.builder
            .app
            .wrap()
            .query_wasm_smart(
                remote_chain_splitter_address.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap();

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            clock_addr,
            splits,
            transfer_amount,
            remote_chain_info,
            app: self.builder.build(),
        }
    }
}


pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub clock_addr: Addr,

    pub splits: BTreeMap<String, SplitConfig>,
    pub transfer_amount: Uint128,
    pub remote_chain_info: RemoteChainInfo,
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

impl Suite {
    pub fn build(mut builder: SuiteBuilder, splitter: Addr) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let split_config: Vec<(String, SplitConfig)> = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::SplitConfig {},
            )
            .unwrap();
        let splits = BTreeMap::from_iter(split_config);

        let transfer_amount = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::TransferAmount {},
            )
            .unwrap();

        let remote_chain_info = builder
            .app
            .wrap()
            .query_wasm_smart(
                splitter.clone(),
                &covenant_remote_chain_splitter::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap();

        Self {
            faucet: builder.faucet.clone(),
            admin: builder.admin.clone(),
            clock_addr,
            splits,
            transfer_amount,
            remote_chain_info,
            app: builder.build(),
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let remote_chain_splitter_addr =
            builder.get_contract_addr(builder.remote_splitter_code_id, REMOTE_CHAIN_SPLITTER_SALT);

        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![remote_chain_splitter_addr.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let party_a_controller_addr = builder.get_random_addr();
        let party_b_controller_addr = builder.get_random_addr();

        let mut splits = BTreeMap::new();
        splits.insert(
            party_a_controller_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );
        splits.insert(
            party_b_controller_addr.to_string(),
            Decimal::from_str("0.5").unwrap(),
        );

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        let remote_chain_splitter_instantiate_msg =
            covenant_remote_chain_splitter::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                remote_chain_connection_id: "connection-0".to_string(),
                remote_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                denom: DENOM_ATOM_ON_NTRN.to_string(),
                amount: Uint128::from(100u128),
                splits: denom_to_split_config_map,
                ibc_fee: IbcFee {
                    recv_fee: vec![coin(1u128, DENOM_ATOM_ON_NTRN)],
                    ack_fee: vec![coin(1u128, DENOM_ATOM_ON_NTRN)],
                    timeout_fee: vec![coin(1u128, DENOM_ATOM_ON_NTRN)],
                },
                ica_timeout: Uint64::from(100u64),
                ibc_transfer_timeout: Uint64::from(100u64),
            };

        builder.contract_init2(
            builder.remote_splitter_code_id,
            REMOTE_CHAIN_SPLITTER_SALT,
            &remote_chain_splitter_instantiate_msg,
            &[],
        );

        Self::build(builder, remote_chain_splitter_addr)
    }
}
