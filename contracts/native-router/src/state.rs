use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::ReceiverConfig;
use cw_storage_plus::Item;

pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const RECEIVER_CONFIG: Item<ReceiverConfig> = Item::new("receiver_config");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
