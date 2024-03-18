use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{Decimal, Uint128, Uint64};
use covenant_utils::split::SplitConfig;

use crate::setup::{DENOM_ATOM_ON_NTRN, NTRN_HUB_CHANNEL};

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
                ica_timeout,
                ibc_transfer_timeout,
            },
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
    pub fn default(clock_address: String, party_a_addr: String, party_b_addr: String) -> Self {
        let mut splits = BTreeMap::new();
        splits.insert(party_a_addr.to_string(), Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_addr.to_string(), Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());

        Self {
            msg: covenant_remote_chain_splitter::msg::InstantiateMsg {
                clock_address,
                remote_chain_connection_id: "connection-0".to_string(),
                remote_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                denom: DENOM_ATOM_ON_NTRN.to_string(),
                amount: Uint128::from(10000u128),
                splits: denom_to_split_config_map,
                ica_timeout: Uint64::from(100u64),
                ibc_transfer_timeout: Uint64::from(100u64),
            },
        }
    }
}
