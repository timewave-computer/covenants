use covenant_utils::{op_mode::ContractOperationMode, split::SplitConfig};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_OP_MODE: Item<ContractOperationMode> = Item::new("contract_op_mode");

/// maps a denom string to a vec of SplitReceivers
pub const SPLIT_CONFIG_MAP: Map<String, SplitConfig> = Map::new("split_config");

/// split for all denoms that are not explicitly defined in SPLIT_CONFIG_MAP
pub const FALLBACK_SPLIT: Item<SplitConfig> = Item::new("fallback_split");
