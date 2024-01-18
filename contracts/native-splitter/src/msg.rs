use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, DepsMut, WasmMsg, StdError, Uint128, Attribute, to_json_binary, Uint64, Decimal};
use covenant_clock::helpers::dequeue_msg;
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_ica_address,
    covenant_remote_chain,
};

use covenant_utils::neutron_ica::RemoteChainInfo;
use neutron_sdk::bindings::msg::IbcFee;
use schemars::Map;

use crate::state::CONTRACT_STATE;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,

    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,
    pub denom: String,
    pub amount: Uint128,

    pub splits: Vec<NativeDenomSplit>,

    /// Neutron requires fees to be set to refund relayers for
    /// submission of ack and timeout messages.
    /// recv_fee and ack_fee paid in untrn from this contract
    pub ibc_fee: IbcFee,
    /// Time in seconds for ICA SubmitTX messages from Neutron
    /// Note that ICA uses ordered channels, a timeout implies
    /// channel closed. We can reopen the channel by reregistering
    /// the ICA with the same port id and connection id
    pub ica_timeout: Uint64,
    /// Timeout in seconds. This is used to craft a timeout timestamp
    /// that will be attached to the IBC transfer message from the ICA
    /// on the host chain (Stride) to its destination. Typically
    /// this timeout should be greater than the ICA timeout, otherwise
    /// if the ICA times out, the destination chain receiving the funds
    /// will also receive the IBC packet with an expired timestamp.
    pub ibc_transfer_timeout: Uint64,
}

#[cw_serde]
pub struct PresetNativeSplitterFields {
    pub code_id: u64,
    pub label: String,
    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,
    pub denom: String,
    pub amount: Uint128,
    pub ibc_fee: IbcFee,
    pub ica_timeout: Uint64,
    pub ibc_transfer_timeout: Uint64,
}

impl PresetNativeSplitterFields {
    pub fn to_instantiate_msg(
        &self,
        clock_address: String,
        splits: Vec<NativeDenomSplit>,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            remote_chain_connection_id: self.remote_chain_connection_id.to_string(),
            remote_chain_channel_id: self.remote_chain_channel_id.to_string(),
            denom: self.denom.to_string(),
            amount: self.amount,
            splits,
            ibc_fee: self.ibc_fee.clone(),
            ica_timeout: self.ica_timeout,
            ibc_transfer_timeout: self.ibc_transfer_timeout,
        }
    }

    pub fn to_instantiate2_msg(
        &self,
        admin_addr: String,
        salt: Binary,
        clock_address: String,
        splits: Vec<NativeDenomSplit>,
    ) -> Result<WasmMsg, StdError> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin_addr),
            code_id: self.code_id,
            label: self.label.to_string(),
            msg: to_json_binary(&self.to_instantiate_msg(clock_address, splits))?,
            funds: vec![],
            salt,
        })
    }
}

#[cw_serde]
pub struct NativeDenomSplit {
    /// denom to be distributed
    pub denom: String,
    /// denom receivers and their respective shares
    // TODO: convert to map of ibc forwarder -> decimal?
    pub receivers: Vec<SplitReceiver>,
}

impl NativeDenomSplit {
    pub fn validate(self) -> Result<NativeDenomSplit, StdError> {
        // here we validate that all receiver shares add up to 100 (%)
        let mut total_share = Decimal::zero();

        self.receivers.iter().for_each(|r| total_share += r.share);

        if total_share != Decimal::one() {
            Err(StdError::generic_err(format!(
                "failed to validate split config for denom: {}",
                self.denom
            )))
        } else {
            Ok(self)
        }
    }

    pub fn to_response_attribute(&self) -> Attribute {
        let mut str = "".to_string();

        for rec in &self.receivers {
            str += rec.to_string().as_str();
        }
        Attribute::new(&self.denom, str)
    }
}

#[cw_serde]
pub struct SplitReceiver {
    /// address of the receiver on remote chain
    pub addr: String,
    /// percentage share that the address is entitled to
    pub share: Decimal,
}

impl fmt::Display for SplitReceiver {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(format!("[{},{}]", self.addr, self.share).as_str())?;
        Ok(())
    }
}
#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
pub struct SplitConfigMap {
    /// maps denom to its associated receivers with their shares
    pub map: Map<String, Vec<SplitReceiver>>,
}

#[covenant_clock_address]
#[covenant_remote_chain]
#[covenant_deposit_address]
#[covenant_ica_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Vec<(String, Vec<SplitReceiver>)>)]
    SplitConfig {},
    #[returns(Uint128)]
    TransferAmount {},
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    IcaCreated,
    Completed,
}

impl ContractState {
    pub fn complete_and_dequeue(deps: DepsMut, clock_addr: &str) -> Result<WasmMsg, StdError> {
        CONTRACT_STATE.save(deps.storage, &ContractState::Completed)?;
        dequeue_msg(clock_addr)
    }
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        remote_chain_info: Option<RemoteChainInfo>,
        splits: Option<Vec<NativeDenomSplit>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
