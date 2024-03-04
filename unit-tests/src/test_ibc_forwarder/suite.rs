use cosmwasm_std::{coin, Addr, Uint128, Uint64};
use covenant_utils::neutron::RemoteChainInfo;
use neutron_sdk::bindings::msg::IbcFee;

use crate::setup::{
    base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp, ASTRO_LIQUID_POOLER_SALT,
    CLOCK_SALT, DENOM_NTRN, IBC_FORWARDER_SALT, NTRN_HUB_CHANNEL,
};

pub(crate) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub clock_addr: Addr,
    pub ibc_forwarder: Addr,
    pub remote_chain_info: RemoteChainInfo,
    pub deposit_address: Option<String>,
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
    pub fn build(mut builder: SuiteBuilder, ibc_forwarder: Addr) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder.clone(),
                &covenant_ibc_forwarder::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let remote_chain_info = builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder.clone(),
                &covenant_ibc_forwarder::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap();

        let deposit_address = builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder.clone(),
                &covenant_ibc_forwarder::msg::QueryMsg::DepositAddress {},
            )
            .unwrap();

        Self {
            app: builder.app,
            faucet: builder.faucet,
            admin: builder.admin,
            clock_addr,
            ibc_forwarder,
            remote_chain_info,
            deposit_address,
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let ibc_forwarder =
            builder.get_contract_addr(builder.ibc_forwarder_code_id, IBC_FORWARDER_SALT);
        let clock_instantiate_msg = covenant_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![ibc_forwarder.to_string()],
        };
        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let ibc_forwarder_instantiate_msg = covenant_ibc_forwarder::msg::InstantiateMsg {
            clock_address: clock_addr.to_string(),
            next_contract: clock_addr.to_string(), // todo
            remote_chain_connection_id: "connection-todo".to_string(),
            remote_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
            denom: DENOM_NTRN.to_string(),
            amount: Uint128::new(100000),
            ibc_fee: IbcFee {
                recv_fee: vec![coin(1u128, DENOM_NTRN)],
                ack_fee: vec![coin(1u128, DENOM_NTRN)],
                timeout_fee: vec![coin(1u128, DENOM_NTRN)],
            },
            ica_timeout: Uint64::from(100u64),
            ibc_transfer_timeout: Uint64::from(100u64),
        };

        builder.contract_init2(
            builder.ibc_forwarder_code_id,
            IBC_FORWARDER_SALT,
            &ibc_forwarder_instantiate_msg,
            &[],
        );

        Self::build(builder, ibc_forwarder)
    }
}
