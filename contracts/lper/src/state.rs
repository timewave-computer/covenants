use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::{AssetData, ContractState, LpConfig, ProvidedLiquidityInfo};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
/// native and ls asset denom information
pub const ASSETS: Item<AssetData> = Item::new("assets");

/// clock contract address to verify the incoming ticks sender
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// holder contract address to verify withdrawal requests
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");

/// keeps track of ls and native token amounts we provided to the pool
pub const PROVIDED_LIQUIDITY_INFO: Item<ProvidedLiquidityInfo> =
    Item::new("provided_liquidity_info");

/// configuration relevant to entering into an LP position
pub const LP_CONFIG: Item<LpConfig> = Item::new("lp_config");
