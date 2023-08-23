use std::{fmt};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64, StdError, Attribute, Fraction};
use covenant_macros::{covenant_deposit_address, clocked, covenant_clock_address, covenant_remote_chain, covenant_ica_address};

use neutron_sdk::bindings::msg::IbcFee;
use covenant_utils::neutron_ica::RemoteChainInfo;
use schemars::Map;

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
pub struct NativeDenomSplit {
    /// denom to be distributed
    pub denom: String,
    /// denom receivers and their respective shares
    pub receivers: Vec<SplitReceiver>,
}

impl NativeDenomSplit {
    pub fn validate(self) -> Result<NativeDenomSplit, StdError> {
        // here we validate that all receiver shares add up to 100 (%)
        let sum: Uint128 = self.receivers.iter().map(|r| r.share).sum();

        if sum != Uint128::new(100) {
            Err(StdError::generic_err(format!("failed to validate split config for denom: {}", self.denom)))
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
    // TODO: convert to cw Fraction
    pub share: Uint128,
}

impl fmt::Display for SplitReceiver {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let str = "[";
        fmt.write_str(str)?;
        fmt.write_str(self.addr.as_str())?;
        fmt.write_str(",")?;
        fmt.write_str(self.share.to_string().as_str())?;
        fmt.write_str("]")?;
        Ok(())
    }
}
#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
pub struct SplitConfigMap {
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