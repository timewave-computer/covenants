
use cosmwasm_std::Addr;
use cw_storage_plus::{Item};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::LPInfo;


// store the clock address to verify calls
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const LP_POSITION: Item<LPInfo> = Item::new("lp_position");
pub const HOLDER_ADDRESS: Item<String> = Item::new("holder_address");
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Instantiated,
    LpPositionEntered,
    LpPositionExited,
    WithdrawComplete,
}

