use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use covenant_clock_derive::clocked;

#[cw_serde]
pub struct InstantiateMsg {
    pub clock_address: String,
    pub stride_neutron_ibc_transfer_channel_id: String,
    pub neutron_stride_ibc_connection_id: String,
    pub lp_address: String,
    pub ls_denom: String,
}

#[cw_serde]
pub struct PresetLsFields {
    pub ls_code: u64,
    pub label: String,
    pub ls_denom: String,
    pub stride_neutron_ibc_transfer_channel_id: String,
    pub neutron_stride_ibc_connection_id: String,
    pub lp_address: String,
}

impl PresetLsFields {
    pub fn to_instantiate_msg(self, clock_address: String) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            stride_neutron_ibc_transfer_channel_id: self.stride_neutron_ibc_transfer_channel_id,
            neutron_stride_ibc_connection_id: self.neutron_stride_ibc_connection_id,
            lp_address: self.lp_address,
            ls_denom: self.ls_denom,
        }
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    Received {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ClockAddress {},
    #[returns(Addr)]
    InterchainAccountAddress {
        interchain_account_id: String,
        connection_id: String,
    },
    #[returns(Addr)]
    StrideICA {},
    #[returns(Addr)]
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
    },
}

#[cw_serde]
pub struct OpenAckVersion {
    pub version: String,
    pub controller_connection_id: String,
    pub host_connection_id: String,
    pub address: String,
    pub encoding: String,
    pub tx_type: String,
}
