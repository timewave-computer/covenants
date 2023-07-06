use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub autopilot_position: String,
    pub clock_address: String,
    pub stride_neutron_ibc_transfer_channel_id: String,
    pub neutron_stride_ibc_connection_id: String,
    pub lp_address: String,
    pub ls_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Tick {},
    Received {},
}

#[cw_serde]
pub enum QueryMsg {
    ClockAddress {},
    InterchainAccountAddress {
        interchain_account_id: String,
        connection_id: String,
    },
    LpAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
  UpdateConfig {
    clock_addr: Option<String>,
    stride_neutron_ibc_transfer_channel_id: Option<String>,
    lp_address: Option<String>,
    neutron_stride_ibc_connection_id: Option<String>,
    ls_denom: Option<String>,
  }
}
