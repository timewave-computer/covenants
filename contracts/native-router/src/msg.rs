use std::collections::BTreeSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, StdResult, WasmMsg};
use covenant_macros::clocked;
use covenant_utils::{instantiate2_helper::Instantiate2HelperConfig, ReceiverConfig};

#[cw_serde]
pub struct InstantiateMsg {
    // List of privileged accounts (if any).
    // The contract's Tick operation can either be a non-privileged (aka permissionless)
    // operation if no privileged accounts are configured (privileged_accounts is None),
    // or a privileged operation, that is, restricted to being executed by one of the configured
    // privileged accounts (when privileged_accounts is Some() with a Vector of one or more addresses).
    pub privileged_accounts: Option<Vec<String>>,
    /// receiver address on local chain
    pub receiver_address: String,
    /// specified denoms to route
    pub denoms: BTreeSet<String>,
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
    DistributeFallback { denoms: Vec<String> },
}

#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    #[returns(ReceiverConfig)]
    ReceiverConfig {},
    #[returns(BTreeSet<String>)]
    TargetDenoms {},
    #[returns(Option<Vec<Addr>>)]
    PrivilegedAccounts {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        privileged_accounts: Option<Option<Vec<String>>>,
        receiver_address: Option<String>,
        target_denoms: Option<Vec<String>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
