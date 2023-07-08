use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::Addr;
use covenant_clock_derive::clocked;
use neutron_sdk::bindings::query::QueryInterchainAccountAddressResponse;

use crate::state::AcknowledgementResult;

#[cw_serde]
pub struct InstantiateMsg {
    pub st_atom_receiver: WeightedReceiver,
    pub atom_receiver: WeightedReceiver,
    pub clock_address: String,
    pub gaia_neutron_ibc_transfer_channel_id: String,
    pub neutron_gaia_connection_id: String,
    pub gaia_stride_ibc_transfer_channel_id: String,
    pub ls_address: String,
    pub ibc_msg_transfer_timeout_timestamp: u64,
}

#[cw_serde]
pub struct WeightedReceiver {
    pub amount: i64,
    pub address: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    Received {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(WeightedReceiver)]
    StAtomReceiver {},
    #[returns(WeightedReceiver)]
    AtomReceiver {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(QueryInterchainAccountAddressResponse)]
    DepositorInterchainAccountAddress {},
    /// this query goes to neutron and get stored ICA with a specific query
    #[returns(QueryInterchainAccountAddressResponse)]
    InterchainAccountAddress {
        interchain_account_id: String,
        connection_id: String,
    },
    // this query returns ICA from contract store, which saved from acknowledgement
    #[returns((String, String))]
    InterchainAccountAddressFromContract {
        interchain_account_id: String,
    },
    // this query returns acknowledgement result after interchain transaction
    #[returns(Option<AcknowledgementResult>)]
    AcknowledgementResult {
        interchain_account_id: String,
        sequence_id: u64,
    },
    // this query returns non-critical errors list
    #[returns(Vec<(Vec<u8>, String)>)]
    ErrorsQueue {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        st_atom_receiver: Option<WeightedReceiver>,
        atom_receiver: Option<WeightedReceiver>,
        gaia_neutron_ibc_transfer_channel_id: Option<String>,
        neutron_gaia_connection_id: Option<String>,
        gaia_stride_ibc_transfer_channel_id: Option<String>,
        ls_address: Option<String>,
    },
    ReregisterICA {},
}
