use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Attribute};
use covenant_clock_derive::clocked;
use covenant_depositor_derive::covenant_deposit_address;
use neutron_sdk::bindings::msg::IbcFee;


#[cw_serde]
pub struct InstantiateMsg {
    /// address for the clock. this contract verifies
    /// that only the clock can execute ticks
    pub clock_address: String,
    /// contract responsible for providing the address to forward the
    /// funds to
    pub next_contract: String,

    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,
    pub denom: String,
    pub amount: String,

    // pub remote_chain_channel_id: String,
    // pub remote_chain_connection_id: String,
    /// neutron requires fees to be set to refund relayers for
    /// submission of ack and timeout messages.
    /// recv_fee and ack_fee paid in untrn from this contract
    pub ibc_fee: IbcFee,
    /// timeout in seconds. this is used to craft a timeout timestamp
    /// that will be attached to the IBC transfer message from the ICA
    /// on the host chain to its destination. typically this timeout
    /// should be greater than the ICA timeout, otherwise if the ICA
    /// times out, the destination chain receiving the funds will also
    /// receive the IBC packet with an expired timestamp.
    pub ibc_transfer_timeout: Uint64,
    /// time in seconds for ICA SubmitTX messages from neutron
    /// note that ICA uses ordered channels, a timeout implies
    /// channel closed. We can reopen the channel by reregistering
    /// the ICA with the same port id and connection id
    pub ica_timeout: Uint64,
}

impl InstantiateMsg {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new("clock_address", &self.clock_address),
            Attribute::new("remote_chain_connection_id", &self.remote_chain_connection_id),
            Attribute::new("remote_chain_channel_id", &self.remote_chain_channel_id),
            Attribute::new("remote_chain_denom", &self.denom),
            Attribute::new("remote_chain_amount", &self.amount),
            Attribute::new("ibc_transfer_timeout", self.ibc_transfer_timeout.to_string()),
            Attribute::new("ica_timeout", self.ica_timeout.to_string()),
        ]
    }
}

#[cw_serde]
pub struct RemoteChainInfo {
    /// connection id from neutron to the remote chain on which
    /// we wish to open an ICA
    pub connection_id: String,
    pub channel_id: String,
    pub denom: String,
    pub amount: String,
    pub ibc_transfer_timeout: Uint64,
    pub ica_timeout: Uint64,
    pub ibc_fee: IbcFee,
}

impl RemoteChainInfo {
    pub fn proto_coin(&self) -> Coin {
        Coin {
            denom: self.denom.to_string(),
            amount: self.amount.to_string(),
        }
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_deposit_address]
#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {}

#[cw_serde]
pub enum ContractState {
    /// Contract was instantiated, ready create ica
    Instantiated,
    /// ICA was created, funds are ready to be forwarded
    ICACreated,
    /// forwarder is complete
    Complete,
}

/// SudoPayload is a type that stores information about a transaction that we try to execute
/// on the host chain. This is a type introduced for our convenience.
#[cw_serde]
pub struct SudoPayload {
    pub message: String,
    pub port_id: String,
}
