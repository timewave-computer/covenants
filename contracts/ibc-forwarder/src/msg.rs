use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, Coin, StdResult, Uint128, Uint64, WasmMsg,
};
use covenant_macros::{
    clocked, covenant_deposit_address, covenant_ica_address, covenant_remote_chain,
};
use covenant_utils::{instantiate2_helper::Instantiate2HelperConfig, neutron::RemoteChainInfo};

#[cw_serde]
pub struct InstantiateMsg {
    // List of privileged addresses (if any).
    // The contract's Tick operation can either be a non-privileged (aka permissionless)
    // operation if no privileged addresses are configured (privileged_accounts is None),
    // or a privileged operation, that is, restricted to being executed by one of the configured
    // privileged addresses (when privileged_accounts is Some() with a Vector of one or more addresses).
    pub privileged_accounts: Option<Vec<String>>,

    /// contract responsible for providing the address to forward the
    /// funds to
    pub next_contract: String,

    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,
    pub denom: String,
    pub amount: Uint128,

    /// timeout in seconds. this is used to craft a timeout timestamp
    /// that will be attached to the IBC transfer message from the ICA
    /// on the host chain to its destination. typically this timeout
    /// should be greater than the ICA timeout, otherwise if the ICA
    /// times out, the destination chain receiving the funds will also
    /// receive the IBC packet with an expired timestamp.
    pub ibc_transfer_timeout: Uint64,
    /// time in seconds for ICA SubmitTX messages from neutron
    /// note that ICA uses ordered channels, a timeout implies
    /// channel closed. We can reopen the channel by reregistering
    /// the ICA with the same port id and connection id
    pub ica_timeout: Uint64,
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

impl InstantiateMsg {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new(
                "privileged_accounts",
                format!("{:?}", self.privileged_accounts),
            ),
            Attribute::new(
                "remote_chain_connection_id",
                &self.remote_chain_connection_id,
            ),
            Attribute::new("remote_chain_channel_id", &self.remote_chain_channel_id),
            Attribute::new("remote_chain_denom", &self.denom),
            Attribute::new("remote_chain_amount", self.amount.to_string()),
            Attribute::new(
                "ibc_transfer_timeout",
                self.ibc_transfer_timeout.to_string(),
            ),
            Attribute::new("ica_timeout", self.ica_timeout.to_string()),
            Attribute::new("fallback_address", format!("{:?}", self.fallback_address)),
        ]
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    DistributeFallback { coins: Vec<Coin> },
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        privileged_accounts: Option<Option<Vec<String>>>,
        next_contract: Option<String>,
        remote_chain_info: Box<Option<RemoteChainInfo>>,
        transfer_amount: Option<Uint128>,
        fallback_address: Option<FallbackAddressUpdateConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}

#[cw_serde]
pub enum FallbackAddressUpdateConfig {
    ExplicitAddress(String),
    Disable {},
}

#[covenant_deposit_address]
#[covenant_remote_chain]
#[covenant_ica_address]
#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Option<String>)]
    FallbackAddress {},
    #[returns(Option<Vec<Addr>>)]
    PrivilegedAddresses {},
}

#[cw_serde]
pub enum ContractState {
    /// Contract was instantiated, ready create ica
    Instantiated,
    /// ICA was created, funds are ready to be forwarded
    IcaCreated,
}
