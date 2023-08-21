use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

use crate::msg::{ContractState, Party};


pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");
pub const WHITELIST_PARTIES: Item<Vec<Party>> = Item::new("whitelist_parties");
pub const EXPIRATION_HEIGHT: Item<u64> = Item::new("expiration_height");
pub const RAGEQUIT_PENALTY: Item<Uint128> = Item::new("ragequit_penalty");