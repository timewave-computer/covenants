
use astroport::asset::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Addr};
use covenant_clock_derive::clocked;

use crate::state::ContractState;

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
pub struct PresetLpFields {
  pub slippage_tolerance: Option<Decimal>,
  pub autostake: Option<bool>,
  pub assets: Vec<Asset>,
  pub lp_code: u64,
  pub lp_position: String,
  pub label: String,
}

impl PresetLpFields {
  pub fn to_instantiate_msg(self, clock_address: String, holder_address: String) -> InstantiateMsg {
    InstantiateMsg { 
      lp_position: LPInfo { addr: self.lp_position },
      clock_address,
      holder_address, 
      slippage_tolerance: self.slippage_tolerance, 
      autostake: self.autostake, 
      assets: self.assets 
    }
  }
}

#[cw_serde]
pub struct LPInfo {
    pub addr: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LPInfo)]
    LpPosition {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Vec<Asset>)]
    Assets {},
}

#[cw_serde]
pub enum MigrateMsg {
  UpdateConfig {
    clock_addr: Option<String>,
    lp_position: Option<LPInfo>,
    holder_address: Option<String>,
  }
}
