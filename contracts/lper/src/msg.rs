
use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct InstantiateMsg {
    pub lp_position: LPInfo,
    pub clock_address: String,
    pub holder_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: Vec<Asset>,
}

#[cw_serde]
pub struct LPInfo {
    pub addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Tick {},
    WithdrawLiquidity {},
}

#[cw_serde]
pub enum QueryMsg {
    LpPosition {},
    ClockAddress {},
    ContractState {},
    HolderAddress {},
    Assets {},
}

#[cw_serde]
pub enum MigrateMsg {
  UpdateConfig {
    clock_addr: Option<String>,
    lp_position: Option<LPInfo>,
    holder_address: Option<String>,
    assets: Option<Vec<Asset>>,
  }
}
