use cosmwasm_std::{Uint128, Uint64};
use neutron_sdk::bindings::msg::IbcFee;

use crate::setup::suite_builder::SuiteBuilder;

pub struct IbcForwarderInstantiate {
    pub msg: covenant_ibc_forwarder::msg::InstantiateMsg,
}

impl From<IbcForwarderInstantiate> for covenant_ibc_forwarder::msg::InstantiateMsg {
    fn from(value: IbcForwarderInstantiate) -> Self {
        value.msg
    }
}

impl IbcForwarderInstantiate {
    pub fn new(
        clock_address: String,
        next_contract: String,
        remote_chain_connection_id: String,
        remote_chain_channel_id: String,
        denom: String,
        amount: Uint128,
        ibc_fee: IbcFee,
        ibc_transfer_timeout: Uint64,
        ica_timeout: Uint64,
    ) -> Self {
        Self {
            msg: covenant_ibc_forwarder::msg::InstantiateMsg {
                clock_address,
                next_contract,
                remote_chain_connection_id,
                remote_chain_channel_id,
                denom,
                amount,
                ibc_fee,
                ibc_transfer_timeout,
                ica_timeout,
            },
        }
    }

    pub fn with_clock_address(&mut self, addr: String) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_next_contract(&mut self, addr: String) -> &mut Self {
        self.msg.next_contract = addr;
        self
    }

    pub fn with_remote_chain_connection_id(&mut self, addr: String) -> &mut Self {
        self.msg.remote_chain_connection_id = addr;
        self
    }

    pub fn with_remote_chain_channel_id(&mut self, addr: String) -> &mut Self {
        self.msg.remote_chain_channel_id = addr;
        self
    }

    pub fn with_denom(&mut self, addr: String) -> &mut Self {
        self.msg.denom = addr;
        self
    }

    pub fn with_amount(&mut self, addr: Uint128) -> &mut Self {
        self.msg.amount = addr;
        self
    }

    pub fn with_ibc_fee(&mut self, addr: IbcFee) -> &mut Self {
        self.msg.ibc_fee = addr;
        self
    }

    pub fn with_ibc_transfer_timeout(&mut self, addr: Uint64) -> &mut Self {
        self.msg.ibc_transfer_timeout = addr;
        self
    }

    pub fn with_ica_timeout(&mut self, addr: Uint64) -> &mut Self {
        self.msg.ica_timeout = addr;
        self
    }
}

impl IbcForwarderInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        clock_address: String,
        next_contract: String,
        remote_chain_connection_id: String,
        remote_chain_channel_id: String,
        denom: String,
        amount: Uint128,
        ibc_fee: IbcFee,
        ibc_transfer_timeout: Uint64,
        ica_timeout: Uint64,
    ) -> Self {
        Self::new(
            clock_address,
            next_contract,
            remote_chain_connection_id,
            remote_chain_channel_id,
            denom,
            amount,
            ibc_fee,
            ibc_transfer_timeout,
            ica_timeout,
        )
    }
}
