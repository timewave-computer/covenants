use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, StdResult, Uint128, Uint64, WasmMsg};
use covenant_macros::{
    clocked, covenant_deposit_address, covenant_ica_address, covenant_remote_chain,
};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, neutron::RemoteChainInfo,
    op_mode::ContractOperationModeConfig,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// IBC transfer channel on Stride for Neutron
    /// This is used to IBC transfer stuatom on Stride
    /// to the LP contract
    pub stride_neutron_ibc_transfer_channel_id: String,
    /// IBC connection ID on Neutron for Stride
    /// We make an Interchain Account over this connection
    pub neutron_stride_ibc_connection_id: String,
    /// Address of the next contract to query for the deposit address
    pub next_contract: String,
    /// The liquid staked denom (e.g., stuatom). This is
    /// required because we only allow transfers of this denom
    /// out of the LSer
    pub ls_denom: String,
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
    // Contract Operation Mode.
    // The contract operation (the Tick function mostly) can either be a permissionless
    // (aka non-privileged) operation, or a permissioned operation, that is,
    // restricted to being executed by one of the configured privileged accounts.
    pub op_mode_cfg: ContractOperationModeConfig,
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
    /// The transfer message allows anybody to permissionlessly
    /// transfer a specified amount of tokens of the preset ls_denom
    /// from the ICA of the host chain to the preset lp_address
    Transfer { amount: Uint128 },
}

#[covenant_remote_chain]
#[covenant_deposit_address]
#[covenant_ica_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(String)]
    NextMemo {},
    #[returns(covenant_utils::op_mode::ContractOperationMode)]
    OperationMode {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        op_mode: Option<ContractOperationModeConfig>,
        next_contract: Option<String>,
        remote_chain_info: Option<RemoteChainInfo>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    IcaCreated,
}
