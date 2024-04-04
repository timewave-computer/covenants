use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, StdResult, WasmMsg};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use covenant_utils::{instantiate2_helper::Instantiate2HelperConfig, split::SplitConfig};

#[cw_serde]
pub struct InstantiateMsg {
    /// address of the associated clock
    pub clock_address: String,
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
    RecoverFunds { denoms: Vec<String> },
}

#[covenant_clock_address]
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
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        fallback_split: Option<SplitConfig>,
        splits: Option<BTreeMap<String, SplitConfig>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
