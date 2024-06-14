use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, Coin, StdResult, Uint128, Uint64, WasmMsg};
use covenant_macros::{
    clocked, covenant_deposit_address, covenant_ica_address, covenant_remote_chain,
};

use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig,
    neutron::RemoteChainInfo,
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    split::SplitConfig,
};

#[cw_serde]
pub struct InstantiateMsg {
    // Contract Operation Mode.
    // The contract operation (the Tick function mostly) can either be a permissionless
    // (aka non-privileged) operation, or a permissioned operation, that is,
    // restricted to being executed by one of the configured privileged accounts.
    pub op_mode_cfg: ContractOperationModeConfig,

    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,
    pub denom: String,
    pub amount: Uint128,

    pub splits: BTreeMap<String, SplitConfig>,

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
    // fallback address on the remote chain
    pub fallback_address: Option<String>,
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
pub enum ExecuteMsg {
    DistributeFallback { coins: Vec<Coin> },
}

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
    #[returns(Option<String>)]
    FallbackAddress {},
    #[returns(ContractOperationMode)]
    OperationMode {},
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    IcaCreated,
}

#[cw_serde]
pub enum FallbackAddressUpdateConfig {
    ExplicitAddress(String),
    Disable {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        op_mode: Option<ContractOperationModeConfig>,
        remote_chain_info: Option<RemoteChainInfo>,
        splits: Option<BTreeMap<String, SplitConfig>>,
        fallback_address: Option<FallbackAddressUpdateConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
