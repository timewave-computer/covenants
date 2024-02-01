use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, DepsMut, StdError, StdResult, Uint128, Uint64, WasmMsg,
};
use covenant_clock::helpers::dequeue_msg;
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_ica_address,
    covenant_remote_chain,
};

use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, neutron::RemoteChainInfo, split::SplitConfig,
};
use neutron_sdk::bindings::msg::IbcFee;

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

    pub splits: BTreeMap<String, SplitConfig>,

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

impl InstantiateMsg {
    pub fn to_instantiate2_msg(
        &self,
        instantiate2_helper: &Instantiate2HelperConfig,
        admin: String,
        label: String,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin),
            code_id: instantiate2_helper.code,
            label,
            msg: to_json_binary(self)?,
            funds: vec![],
            salt: instantiate2_helper.salt.clone(),
        })
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_clock_address]
#[covenant_remote_chain]
#[covenant_deposit_address]
#[covenant_ica_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Vec<(String, SplitConfig)>)]
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
        splits: Option<BTreeMap<String, SplitConfig>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
