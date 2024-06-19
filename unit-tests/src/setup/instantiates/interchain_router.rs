use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_std::{Addr, Uint64};
use covenant_utils::{op_mode::ContractOperationModeConfig, DestinationConfig};

use crate::setup::{DENOM_ATOM_ON_NTRN, NTRN_HUB_CHANNEL};

pub struct InterchainRouterInstantiate {
    pub msg: valence_interchain_router::msg::InstantiateMsg,
}

impl From<InterchainRouterInstantiate> for valence_interchain_router::msg::InstantiateMsg {
    fn from(value: InterchainRouterInstantiate) -> Self {
        value.msg
    }
}

impl InterchainRouterInstantiate {
    pub fn new(
        clock_address: Addr,
        destination_config: DestinationConfig,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self {
            msg: valence_interchain_router::msg::InstantiateMsg {
                op_mode_cfg: ContractOperationModeConfig::Permissioned(vec![
                    clock_address.to_string()
                ]),
                destination_config,
                denoms,
            },
        }
    }

    pub fn with_op_mode(&mut self, op_mode: ContractOperationModeConfig) -> &mut Self {
        self.msg.op_mode_cfg = op_mode;
        self
    }

    pub fn with_destination_config(&mut self, destination_config: DestinationConfig) -> &mut Self {
        self.msg.destination_config = destination_config;
        self
    }

    pub fn with_denoms(&mut self, denoms: BTreeSet<String>) -> &mut Self {
        self.msg.denoms = denoms;
        self
    }
}

impl InterchainRouterInstantiate {
    pub fn default(clock_address: Addr, party_receiver: String) -> Self {
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        let destination_config = DestinationConfig {
            local_to_destination_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
            destination_receiver_addr: party_receiver,
            ibc_transfer_timeout: Uint64::new(1000),
            denom_to_pfm_map: BTreeMap::new(),
        };

        Self::new(clock_address, destination_config, denoms)
    }
}
