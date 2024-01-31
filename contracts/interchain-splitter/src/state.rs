use cosmwasm_std::Addr;
use covenant_utils::split::SplitConfig;
use cw_storage_plus::{Item, Map};

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

/// maps a denom string to a vec of SplitReceivers
pub const SPLIT_CONFIG_MAP: Map<String, SplitConfig> = Map::new("split_config");

/// split for all denoms that are not explicitly defined in SPLIT_CONFIG_MAP
pub const FALLBACK_SPLIT: Item<SplitConfig> = Item::new("fallback_split");
