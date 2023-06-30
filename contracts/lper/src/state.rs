use astroport::asset::Asset;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::LPInfo;


// store the clock address to verify calls
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const LP_POSITION: Item<LPInfo> = Item::new("lp_position");
pub const HOLDER_ADDRESS: Item<String> = Item::new("holder_address");
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const AUTOSTAKE: Item<Option<bool>> = Item::new("autostake");
pub const SLIPPAGE_TOLERANCE: Item<Option<Decimal>> = Item::new("slippage_tolerance");
pub const ASSETS: Item<Vec<Asset>> = Item::new("assets");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Instantiated,
    LpPositionEntered,
    LpPositionExited,
    WithdrawComplete,
}

