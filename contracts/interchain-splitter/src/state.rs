use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");


// maps a denom string to a vec of SplitReceivers
// pub const SPLIT_CONFIG_MAP: Map<String, Vec<SplitReceiver>> = Map::new("split_config");
