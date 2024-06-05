use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::op_mode::ContractOperationMode;
use cw_storage_plus::Item;

pub const CONTRACT_OP_MODE: Item<ContractOperationMode> = Item::new("contract_op_mode");
pub const RECEIVER_ADDRESS: Item<Addr> = Item::new("receiver_address");
pub const TARGET_DENOMS: Item<BTreeSet<String>> = Item::new("denoms");
