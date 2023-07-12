use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use covenant_clock::msg::PresetClockFields;
use covenant_depositor::msg::PresetDepositorFields;
use covenant_holder::msg::PresetHolderFields;
use covenant_lp::msg::PresetLpFields;
use covenant_ls::msg::PresetLsFields;

#[cw_serde]
pub struct InstantiateMsg {
    pub label: String,
    pub preset_clock_fields: PresetClockFields,
    pub preset_ls_fields: PresetLsFields,
    pub preset_depositor_fields: PresetDepositorFields,
    pub preset_lp_fields: PresetLpFields,
    pub preset_holder_fields: PresetHolderFields,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    DepositorAddress {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(Addr)]
    LpAddress {},
    #[returns(Addr)]
    LsAddress {},
    #[returns(Addr)]
    HolderAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        depositor: Option<covenant_depositor::msg::MigrateMsg>,
        lp: Option<covenant_lp::msg::MigrateMsg>,
        ls: Option<covenant_ls::msg::MigrateMsg>,
        holder: Option<covenant_holder::msg::MigrateMsg>,
    },
}
