use std::collections::HashSet;

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::{ContractState, LpConfig, ProvidedLiquidityInfo};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

pub const PRIVILEGED_ACCOUNTS: Item<Option<HashSet<Addr>>> = Item::new("privileged_accounts");

/// holder module address to verify withdrawal requests
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");

/// keeps track of both token amounts we provided to the pool
pub const PROVIDED_LIQUIDITY_INFO: Item<ProvidedLiquidityInfo> =
    Item::new("provided_liquidity_info");

/// configuration relevant to entering into an LP position
pub const LP_CONFIG: Item<LpConfig> = Item::new("lp_config");
