use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::Addr;
use covenant_ls::msg::InstantiateMsg as LsInstantiateMsg;
use covenant_depositor::msg::InstantiateMsg as DepositorInstantiateMsg;
use covenant_lp::msg::InstantiateMsg as LpInstantiateMsg;
use covenant_clock::msg::InstantiateMsg as ClockInstantiateMsg;
use covenant_holder::msg::InstantiateMsg as HolderInstantiateMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub clock_code: u64,
    pub clock_instantiate: ClockInstantiateMsg,
    pub ls_code: u64,
    pub ls_instantiate: LsInstantiateMsg,
    pub depositor_code: u64,
    pub depositor_instantiate: DepositorInstantiateMsg,
    pub lp_code: u64,
    pub lp_instantiate: LpInstantiateMsg,
    pub holder_code: u64,
    pub holder_instantiate: HolderInstantiateMsg,
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
pub struct MigrateMsg {}