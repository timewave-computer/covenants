use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::{ContractState, PartiesConfig, LockupConfig};


pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");
pub const PARTIES_CONFIG: Item<PartiesConfig> = Item::new("parties_config");
pub const LOCKUP_CONFIG: Item<LockupConfig> = Item::new("lockup_config");