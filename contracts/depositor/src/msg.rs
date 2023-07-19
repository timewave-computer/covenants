use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use covenant_clock_derive::clocked;
use neutron_sdk::bindings::{msg::IbcFee, query::QueryInterchainAccountAddressResponse};

use crate::state::{AcknowledgementResult, ContractState};

#[cw_serde]
pub struct InstantiateMsg {
    pub st_atom_receiver: WeightedReceiver,
    pub atom_receiver: WeightedReceiver,
    pub clock_address: String,
    pub gaia_neutron_ibc_transfer_channel_id: String,
    pub neutron_gaia_connection_id: String,
    pub gaia_stride_ibc_transfer_channel_id: String,
    pub ls_address: String,
    pub autopilot_format: String,
    pub ibc_timeout: u64,
    pub ibc_fee: IbcFee,
}

#[cw_serde]
pub struct PresetDepositorFields {
    pub gaia_neutron_ibc_transfer_channel_id: String,
    pub neutron_gaia_connection_id: String,
    pub gaia_stride_ibc_transfer_channel_id: String,
    pub depositor_code: u64,
    pub label: String,
    pub st_atom_receiver_amount: WeightedReceiverAmount,
    pub atom_receiver_amount: WeightedReceiverAmount,
    pub autopilot_format: String,
}

#[cw_serde]
pub struct WeightedReceiverAmount {
    pub amount: i64,
}

impl WeightedReceiverAmount {
    pub fn to_weighted_receiver(self, addr: String) -> WeightedReceiver {
        WeightedReceiver {
            amount: self.amount,
            address: addr,
        }
    }
}

impl PresetDepositorFields {
    pub fn to_instantiate_msg(
        self,
        st_atom_receiver_addr: String,
        clock_address: String,
        ls_address: String,
        lp_address: String,
        ibc_timeout: u64,
        ibc_fee: IbcFee,
    ) -> InstantiateMsg {
        InstantiateMsg {
            st_atom_receiver: self
                .st_atom_receiver_amount
                .to_weighted_receiver(st_atom_receiver_addr),
            atom_receiver: self.atom_receiver_amount.to_weighted_receiver(lp_address),
            clock_address,
            gaia_neutron_ibc_transfer_channel_id: self.gaia_neutron_ibc_transfer_channel_id,
            neutron_gaia_connection_id: self.neutron_gaia_connection_id,
            gaia_stride_ibc_transfer_channel_id: self.gaia_stride_ibc_transfer_channel_id,
            ls_address,
            autopilot_format: self.autopilot_format,
            ibc_timeout,
            ibc_fee,
        }
    }
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
    #[returns(ContractState)]
    ContractState {},
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
    InterchainAccountAddressFromContract { interchain_account_id: String },
    // this query returns acknowledgement result after interchain transaction
    #[returns(Option<AcknowledgementResult>)]
    AcknowledgementResult {
        interchain_account_id: String,
        sequence_id: u64,
    },
    // this query returns non-critical errors list
    #[returns(Vec<(Vec<u8>, String)>)]
    ErrorsQueue {},
    #[returns(String)]
    AutopilotFormat {},
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
        autopilot_format: Option<String>,
        ibc_timeout: Option<u64>,
        ibc_fee: Option<IbcFee>,
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
