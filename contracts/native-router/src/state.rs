use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const RECEIVER_ADDRESS: Item<Addr> = Item::new("receiver_address");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
