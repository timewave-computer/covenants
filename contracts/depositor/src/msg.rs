use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub st_atom_receiver: WeightedReceiver,
    pub atom_receiver: WeightedReceiver,
    pub clock_address: String,
    pub gaia_neutron_ibc_transfer_channel_id: String,
    pub neutron_gaia_connection_id: String,
    pub gaia_stride_ibc_transfer_channel_id: String,
}

#[cw_serde]
pub struct WeightedReceiver {
    pub amount: i64,
    pub address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Tick {},
    Received {},
}

#[cw_serde]
pub enum QueryMsg {
    StAtomReceiver {},
    AtomReceiver {},
    ClockAddress {},
    DepositorInterchainAccountAddress {},
    /// this query goes to neutron and get stored ICA with a specific query
    InterchainAccountAddress {
        interchain_account_id: String,
        connection_id: String,
    },
    // this query returns ICA from contract store, which saved from acknowledgement
    InterchainAccountAddressFromContract {
        interchain_account_id: String,
    },
    // this query returns acknowledgement result after interchain transaction
    AcknowledgementResult {
        interchain_account_id: String,
        sequence_id: u64,
    },
    // this query returns non-critical errors list
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
    },
}
