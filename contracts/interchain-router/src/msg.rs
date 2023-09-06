use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Addr, Attribute, Binary, Coin, CosmosMsg, IbcMsg, IbcTimeout, Timestamp, Uint64,
};
use covenant_macros::{clocked, covenant_clock_address};

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

#[cw_serde]
pub struct DestinationConfig {
    /// channel id of the destination chain
    pub destination_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub destination_receiver_addr: Addr,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
}

impl DestinationConfig {
    pub fn get_ibc_transfer_messages_for_coins(
        &self,
        coins: Vec<Coin>,
        current_timestamp: Timestamp,
    ) -> Vec<CosmosMsg> {
        let mut messages: Vec<CosmosMsg> = vec![];

        for coin in coins {
            let msg: IbcMsg = IbcMsg::Transfer {
                channel_id: self.destination_chain_channel_id.to_string(),
                to_address: self.destination_receiver_addr.to_string(),
                amount: coin,
                timeout: IbcTimeout::with_timestamp(
                    current_timestamp.plus_seconds(self.ibc_transfer_timeout.u64()),
                ),
            };

            messages.push(CosmosMsg::Ibc(msg));
        }

        messages
    }

    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new(
                "destination_chain_channel_id",
                self.destination_chain_channel_id.to_string(),
            ),
            Attribute::new(
                "destination_receiver_addr",
                self.destination_receiver_addr.to_string(),
            ),
            Attribute::new("ibc_transfer_timeout", self.ibc_transfer_timeout),
        ]
    }
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
