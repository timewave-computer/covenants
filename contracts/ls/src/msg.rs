use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128, Uint64};
use covenant_clock_derive::clocked;
use neutron_sdk::bindings::msg::IbcFee;

use crate::state::ContractState;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// IBC transfer channel on Stride for Neutron
    /// This is used to IBC transfer stuatom on Stride 
    /// to the LP contract
    pub stride_neutron_ibc_transfer_channel_id: String,
    /// IBC connection ID on Neutron for Stride
    /// We make an Interchain Account over this connection
    pub neutron_stride_ibc_connection_id: String,
    /// Address for the covenant's LP contract.
    /// We send the liquid staked amount to this address
    pub lp_address: String,
    /// The liquid staked denom (e.g., stuatom). This is
    /// required because we only allow transfers of this denom
    /// out of the LSer
    pub ls_denom: String,
    pub ibc_fee: IbcFee,
    pub ica_timeout: Uint64,
    pub ibc_transfer_timeout: Uint64,
}

#[cw_serde]
pub struct PresetLsFields {
    pub ls_code: u64,
    pub label: String,
    pub ls_denom: String,
    pub stride_neutron_ibc_transfer_channel_id: String,
    pub neutron_stride_ibc_connection_id: String,
}

impl PresetLsFields {
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        lp_address: String,
        ibc_fee: IbcFee,
        ica_timeout: Uint64,
        ibc_transfer_timeout: Uint64,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            stride_neutron_ibc_transfer_channel_id: self.stride_neutron_ibc_transfer_channel_id,
            neutron_stride_ibc_connection_id: self.neutron_stride_ibc_connection_id,
            lp_address,
            ls_denom: self.ls_denom,
            ibc_fee,
            ica_timeout,
            ibc_transfer_timeout,
        }
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    Received {},
    Transfer { amount: Uint128 },
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
    #[returns(ContractState)]
    ContractState {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        stride_neutron_ibc_transfer_channel_id: Option<String>,
        lp_address: Option<String>,
        neutron_stride_ibc_connection_id: Option<String>,
        ls_denom: Option<String>,
        ibc_fee: Option<IbcFee>,
        ibc_transfer_timeout: Option<Uint64>,
        ica_timeout: Option<Uint64>,
    },
    UpdateCodeId {
        data: Option<Binary>,
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
