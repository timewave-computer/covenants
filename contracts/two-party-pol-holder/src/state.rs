use cosmwasm_std::Addr;
use covenant_utils::LockupConfig;
use cw_storage_plus::Item;

use crate::msg::{ContractState, RagequitConfig, TwoPartyPolCovenantConfig};


pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

pub const LOCKUP_CONFIG: Item<LockupConfig> = Item::new("lockup_config");
pub const RAGEQUIT_CONFIG: Item<RagequitConfig> = Item::new("ragequit_config");

pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");

pub const DEPOSIT_DEADLINE: Item<LockupConfig> = Item::new("deposit_deadline");

pub const PARTY_A_ROUTER: Item<Addr> = Item::new("party_a_router");
pub const PARTY_B_ROUTER: Item<Addr> = Item::new("party_b_router");

pub const COVENANT_CONFIG: Item<TwoPartyPolCovenantConfig> = Item::new("covenant_config");
