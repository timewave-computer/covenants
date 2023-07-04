use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use covenant_ls::msg::InstantiateMsg as LsInstantiateMsg;
use covenant_depositor::msg::InstantiateMsg as DepositorInstantiateMsg;
use covenant_lp::msg::InstantiateMsg as LpInstantiateMsg;
use covenant_clock::msg::InstantiateMsg as ClockInstantiateMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub clock_code: u64,
    pub clock_instantiate: ClockInstantiateMsg,
    pub ls_code: u64,
    pub ls_instantiate: LsInstantiateMsg,
    pub depositor_code: u64,
    pub depositor_instantiate: DepositorInstantiateMsg,
    pub lp_code: u64,
    pub lp_instantiate: LpInstantiateMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}