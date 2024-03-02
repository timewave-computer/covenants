use std::collections::BTreeMap;

use cosmwasm_std::{Uint128, Uint64};
use covenant_utils::split::SplitConfig;
use neutron_sdk::bindings::msg::IbcFee;

use crate::setup::suite_builder::SuiteBuilder;

use super::clock;


pub struct RemoteChainSplitterInstantiate {
    pub msg: covenant_remote_chain_splitter::msg::InstantiateMsg,
}
impl From<RemoteChainSplitterInstantiate> for covenant_remote_chain_splitter::msg::InstantiateMsg {
    fn from(value: RemoteChainSplitterInstantiate) -> Self {
        value.msg
    }
}

impl RemoteChainSplitterInstantiate {
    pub fn new(
        clock_address: String,
        remote_chain_connection_id: String,
        remote_chain_channel_id: String,
        denom: String,
        amount: Uint128,
        splits: BTreeMap<String, SplitConfig>,
        ibc_fee: IbcFee,
        ica_timeout: Uint64,
        ibc_transfer_timeout: Uint64,
    ) -> Self {
        Self {
            msg: covenant_remote_chain_splitter::msg::InstantiateMsg {
                clock_address,
                remote_chain_connection_id,
                remote_chain_channel_id,
                denom,
                amount,
                splits,
                ibc_fee,
                ica_timeout,
                ibc_transfer_timeout,
            }
        }
    }

    pub fn with_clock_address(&mut self, addr: String) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_remote_chain_connection_id(&mut self, id: String) -> &mut Self {
        self.msg.remote_chain_connection_id = id;
        self
    }

    pub fn with_remote_chain_channel_id(&mut self, id: String) -> &mut Self {
        self.msg.remote_chain_channel_id = id;
        self
    }

    pub fn with_denom(&mut self, denom: String) -> &mut Self {
        self.msg.denom = denom;
        self
    }

    pub fn with_amount(&mut self, amount: Uint128) -> &mut Self {
        self.msg.amount = amount;
        self
    }

    pub fn with_splits(&mut self, splits: BTreeMap<String, SplitConfig>) -> &mut Self {
        self.msg.splits = splits;
        self
    }

    pub fn with_ibc_fee(&mut self, ibc_fee: IbcFee) -> &mut Self {
        self.msg.ibc_fee = ibc_fee;
        self
    }

    pub fn with_ica_timeout(&mut self, ica_timeout: Uint64) -> &mut Self {
        self.msg.ica_timeout = ica_timeout;
        self
    }

    pub fn with_ibc_transfer_timeout(&mut self, ibc_transfer_timeout: Uint64) -> &mut Self {
        self.msg.ibc_transfer_timeout = ibc_transfer_timeout;
        self
    }
}

impl RemoteChainSplitterInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        clock_address: String,
        splits: BTreeMap<String, SplitConfig>,
        remote_chain_connection_id: String,
        remote_chain_channel_id: String,
        denom: String,
        amount: Uint128,
        ibc_fee: IbcFee,
        ica_timeout: Uint64,
        ibc_transfer_timeout: Uint64,
    ) -> Self {
        Self::new(
            clock_address,
            remote_chain_connection_id,
            remote_chain_channel_id,
            denom,
            amount,
            splits,
            ibc_fee,
            ica_timeout,
            ibc_transfer_timeout,
        )
    }
}
