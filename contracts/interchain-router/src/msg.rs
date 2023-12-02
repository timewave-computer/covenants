use std::collections::BTreeSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint64};
use covenant_macros::{clocked, covenant_clock_address};
use covenant_utils::DestinationConfig;

#[cw_serde]
pub struct InstantiateMsg {
    /// address for the clock. this contract verifies
    /// that only the clock can execute ticks
    pub clock_address: String,
    /// channel id of the destination chain
    pub destination_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub destination_receiver_addr: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
    /// specified denoms to route
    pub denoms: BTreeSet<String>,
}

#[cw_serde]
pub struct PresetInterchainRouterFields {
    /// channel id of the destination chain
    pub destination_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub destination_receiver_addr: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
    /// specified denoms to route
    pub denoms: BTreeSet<String>,
    pub label: String,
    pub code_id: u64,
}

impl PresetInterchainRouterFields {
    pub fn to_instantiate_msg(&self, clock_address: String) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            destination_chain_channel_id: self.destination_chain_channel_id.to_string(),
            destination_receiver_addr: self.destination_receiver_addr.to_string(),
            ibc_transfer_timeout: self.ibc_transfer_timeout,
            denoms: self.denoms.clone(),
        }
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    DistributeFallback { denoms: Vec<String> },
}

#[covenant_clock_address]
#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    #[returns(DestinationConfig)]
    DestinationConfig {},
    #[returns(BTreeSet<String>)]
    TargetDenoms {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        destination_config: Option<DestinationConfig>,
        target_denoms: Option<Vec<String>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
