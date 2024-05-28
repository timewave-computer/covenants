use std::collections::{BTreeSet, HashSet};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const PRIVILEGED_ACCOUNTS: Item<Option<HashSet<Addr>>> = Item::new("privileged_accounts");
pub const RECEIVER_ADDRESS: Item<Addr> = Item::new("receiver_address");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
