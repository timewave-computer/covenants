use std::collections::BTreeSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, StdResult, WasmMsg};
use covenant_macros::{clocked, covenant_clock_address};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, DestinationConfig, ReceiverConfig,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// address for the clock. this contract verifies
    /// that only the clock can execute ticks
    pub clock_address: Addr,
    /// config that determines how to facilitate the ibc routing
    pub destination_config: DestinationConfig,
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

#[covenant_clock_address]
#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    #[returns(ReceiverConfig)]
    ReceiverConfig {},
    #[returns(BTreeSet<String>)]
    TargetDenoms {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        destination_config: Option<DestinationConfig>,
        target_denoms: Option<Vec<String>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
