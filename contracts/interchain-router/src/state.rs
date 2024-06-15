use std::collections::BTreeSet;

use covenant_utils::{op_mode::ContractOperationMode, DestinationConfig};
use cw_storage_plus::Item;

pub const CONTRACT_OP_MODE: Item<ContractOperationMode> = Item::new("contract_op_mode");
pub const DESTINATION_CONFIG: Item<DestinationConfig> = Item::new("destination_config");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
