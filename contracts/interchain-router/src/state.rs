use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::ReceiverConfig;
use cw_storage_plus::Item;

pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const DESTINATION_CONFIG: Item<DestinationConfig> = Item::new("destination_config");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
