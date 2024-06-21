use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, StdResult, WasmMsg};
use covenant_macros::{clocked, covenant_deposit_address};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig,
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
    /// maps denom to its split configuration
    pub splits: BTreeMap<String, SplitConfig>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitConfig>,
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

#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SplitConfig)]
    DenomSplit { denom: String },
    #[returns(Vec<(String, SplitConfig)>)]
    Splits {},
    #[returns(SplitConfig)]
    FallbackSplit {},
    #[returns(ContractOperationMode)]
    OperationMode {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        op_mode: Option<ContractOperationModeConfig>,
        fallback_split: Option<SplitConfig>,
        splits: Option<BTreeMap<String, SplitConfig>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
