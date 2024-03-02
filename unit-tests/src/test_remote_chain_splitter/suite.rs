use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128, Uint64};
use covenant_utils::{neutron::RemoteChainInfo, split::SplitConfig};
use neutron_sdk::bindings::msg::IbcFee;

use crate::setup::{base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp, CLOCK_SALT, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, NTRN_HUB_CHANNEL, REMOTE_CHAIN_SPLITTER_SALT};


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
}

impl Suite {
    pub fn build(
        mut builder: SuiteBuilder,
        splitter: Addr,
    ) -> Self {

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
            faucet: builder.fuacet.clone(),
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


        let clock_addr = builder.get_contract_addr(
            builder.clock_code_id,
            CLOCK_SALT,
        );
        let remote_chain_splitter_addr = builder.get_contract_addr(
            builder.remote_splitter_code_id,
            REMOTE_CHAIN_SPLITTER_SALT,
        );

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
        splits.insert(party_a_controller_addr.to_string(), Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_controller_addr.to_string(), Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig {
            receivers: splits,
        };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        let remote_chain_splitter_instantiate_msg = covenant_remote_chain_splitter::msg::InstantiateMsg {
            clock_address: clock_addr.to_string(),
            remote_chain_connection_id: "connection-0".to_string(),
            remote_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
            denom: DENOM_ATOM_ON_NTRN.to_string(),
            amount: Uint128::from(100u128),
            splits: denom_to_split_config_map,
            ibc_fee: IbcFee {
                recv_fee: vec![
                    coin(1u128, DENOM_ATOM_ON_NTRN),
                ],
                ack_fee: vec![
                    coin(1u128, DENOM_ATOM_ON_NTRN),
                ],
                timeout_fee: vec![
                    coin(1u128, DENOM_ATOM_ON_NTRN),
                ],
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

        Self::build(
            builder,
            remote_chain_splitter_addr,
        )
    }
}
