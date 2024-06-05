use std::str::FromStr;

use cosmwasm_std::{Addr, Coin, Uint128, Uint64};
use covenant_utils::{
    neutron::RemoteChainInfo,
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
};
use cw_multi_test::{AppResponse, Executor};
use cw_storage_plus::KeyDeserialize;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::ibc_forwarder::IbcForwarderInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, CLOCK_SALT, IBC_FORWARDER_SALT,
};

pub struct IbcForwarderBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: IbcForwarderInstantiate,
    pub clock_addr: Addr,
}

#[allow(dead_code)]
impl IbcForwarderBuilder {
    pub fn default() -> Self {
        let mut builder = SuiteBuilder::new();

        let clock_addr = builder.get_contract_addr(builder.clock_code_id, CLOCK_SALT);
        let ibc_forwarder_addr =
            builder.get_contract_addr(builder.ibc_forwarder_code_id, IBC_FORWARDER_SALT);
        let next_contract_addr =
            builder.get_contract_addr(builder.ibc_forwarder_code_id, "deposit_forwarder");

        let clock_instantiate_msg = valence_clock::msg::InstantiateMsg {
            tick_max_gas: None,
            whitelist: vec![
                ibc_forwarder_addr.to_string(),
                next_contract_addr.to_string(),
            ],
            initial_queue: vec![],
        };

        builder.contract_init2(
            builder.clock_code_id,
            CLOCK_SALT,
            &clock_instantiate_msg,
            &[],
        );

        let next_contract_instantiate = IbcForwarderInstantiate::default(
            ContractOperationModeConfig::Permissioned(vec![clock_addr.to_string()]),
            clock_addr.to_string(),
            Some(builder.get_random_addr().to_string()),
        );
        builder.contract_init2(
            builder.ibc_forwarder_code_id,
            "deposit_forwarder",
            &next_contract_instantiate.msg,
            &[],
        );

        let ibc_forwarder_instantiate = IbcForwarderInstantiate::default(
            ContractOperationModeConfig::Permissioned(vec![clock_addr.to_string()]),
            next_contract_addr.to_string(),
            Some(builder.get_random_addr().to_string()),
        );

        IbcForwarderBuilder {
            builder,
            instantiate_msg: ibc_forwarder_instantiate,
            clock_addr,
        }
    }

    pub fn with_denom(mut self, denom: String) -> Self {
        self.instantiate_msg.with_denom(denom);
        self
    }

    pub fn with_amount(mut self, amount: Uint128) -> Self {
        self.instantiate_msg.with_amount(amount);
        self
    }

    pub fn with_fallback_address(mut self, fallback_address: String) -> Self {
        self.instantiate_msg.with_fallback_address(fallback_address);
        self
    }

    pub fn with_ibc_transfer_timeout(mut self, ibc_transfer_timeout: Uint64) -> Self {
        self.instantiate_msg
            .with_ibc_transfer_timeout(ibc_transfer_timeout);
        self
    }

    pub fn with_ica_timeout(mut self, ica_timeout: Uint64) -> Self {
        self.instantiate_msg.with_ica_timeout(ica_timeout);
        self
    }

    pub fn with_next_contract(mut self, next_contract: String) -> Self {
        self.instantiate_msg.with_next_contract(next_contract);
        self
    }

    pub fn with_op_mode(mut self, op_mode: ContractOperationModeConfig) -> Self {
        self.instantiate_msg.with_op_mode(op_mode);
        self
    }

    pub fn with_remote_chain_connection_id(mut self, remote_chain_connection_id: String) -> Self {
        self.instantiate_msg
            .with_remote_chain_connection_id(remote_chain_connection_id);
        self
    }

    pub fn with_remote_chain_channel_id(mut self, remote_chain_channel_id: String) -> Self {
        self.instantiate_msg
            .with_remote_chain_channel_id(remote_chain_channel_id);
        self
    }

    pub fn build(mut self) -> Suite {
        let ibc_forwarder_address = self.builder.contract_init2(
            self.builder.ibc_forwarder_code_id,
            IBC_FORWARDER_SALT,
            &self.instantiate_msg.msg,
            &[],
        );

        let op_mode: ContractOperationMode = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder_address.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::OperationMode {},
            )
            .unwrap();

        let remote_chain_info = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder_address.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap();

        let deposit_address = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder_address.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::DepositAddress {},
            )
            .unwrap();

        let fallback_address = self
            .builder
            .app
            .wrap()
            .query_wasm_smart(
                ibc_forwarder_address.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::FallbackAddress {},
            )
            .unwrap();

        Suite {
            app: self.builder.app,
            faucet: self.builder.faucet,
            admin: self.builder.admin,
            clock_addr: self.clock_addr,
            op_mode,
            ibc_forwarder: ibc_forwarder_address,
            remote_chain_info,
            deposit_address,
            fallback_address,
        }
    }
}

#[allow(dead_code)]
pub struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub clock_addr: Addr,
    pub op_mode: ContractOperationMode,
    pub ibc_forwarder: Addr,
    pub remote_chain_info: RemoteChainInfo,
    pub deposit_address: Option<String>,
    pub fallback_address: Option<Addr>,
}

impl Suite {
    pub(crate) fn query_deposit_address(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::DepositAddress {},
            )
            .unwrap()
    }

    pub(crate) fn query_remote_chain_info(&mut self) -> RemoteChainInfo {
        self.app
            .wrap()
            .query_wasm_smart(
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::RemoteChainInfo {},
            )
            .unwrap()
    }

    pub(crate) fn query_op_mode(&mut self) -> ContractOperationMode {
        self.app
            .wrap()
            .query_wasm_smart(
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::OperationMode {},
            )
            .unwrap()
    }

    pub(crate) fn query_contract_state(&mut self) -> valence_ibc_forwarder::msg::ContractState {
        self.app
            .wrap()
            .query_wasm_smart(
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::ContractState {},
            )
            .unwrap()
    }

    pub(crate) fn query_ica_address(&mut self, addr: Addr) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(addr, &valence_ibc_forwarder::msg::QueryMsg::IcaAddress {})
            .unwrap()
    }

    // temp fix until we add a query
    pub(crate) fn query_next_contract(&mut self) -> Addr {
        let resp = self
            .app
            .wrap()
            .query_wasm_raw(self.ibc_forwarder.clone(), "next_contract".as_bytes())
            .unwrap();

        let mut val = resp.unwrap().split_off(1);
        val.truncate(val.len() - 1);
        Addr::from_slice(&val).unwrap()
    }

    pub fn query_fallback_address(&mut self) -> Option<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::QueryMsg::FallbackAddress {},
            )
            .unwrap()
    }

    // temp fix until we add a query
    pub(crate) fn query_transfer_amount(&mut self) -> Uint128 {
        let resp = self
            .app
            .wrap()
            .query_wasm_raw(self.ibc_forwarder.clone(), "transfer_amount".as_bytes())
            .unwrap();

        let mut val = resp.unwrap().split_off(1);
        val.truncate(val.len() - 1);

        let transfer_amount = String::from_vec(val).unwrap();

        Uint128::from_str(&transfer_amount).unwrap()
    }

    pub fn query_admin_address(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_contract_info(self.ibc_forwarder.to_string())
            .unwrap()
            .admin
            .unwrap()
    }

    pub fn distribute_fallback(&mut self, coins: Vec<Coin>, funds: Vec<Coin>) -> AppResponse {
        self.app
            .execute_contract(
                self.faucet.clone(),
                self.ibc_forwarder.clone(),
                &valence_ibc_forwarder::msg::ExecuteMsg::DistributeFallback { coins },
                &funds,
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
