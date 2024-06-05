use std::collections::BTreeSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, StdResult, WasmMsg};
use covenant_macros::clocked;
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig,
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    ReceiverConfig,
};

#[cw_serde]
pub struct InstantiateMsg {
    // Contract Operation Mode.
    // The contract operation (the Tick function mostly) can either be a permissionless
    // (aka non-privileged) operation, or a permissioned operation, that is,
    // restricted to being executed by one of the configured privileged accounts.
    pub op_mode_cfg: ContractOperationModeConfig,
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
    #[returns(ContractOperationMode)]
    OperationMode {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        op_mode: Option<ContractOperationModeConfig>,
        receiver_address: Option<String>,
        target_denoms: Option<Vec<String>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
