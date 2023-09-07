use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Addr, Attribute, Binary, Coin, CosmosMsg, IbcMsg, IbcTimeout, Timestamp, Uint64,
};
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
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_clock_address]
#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    #[returns(DestinationConfig)]
    DestinationConfig {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        destination_config: Option<DestinationConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
