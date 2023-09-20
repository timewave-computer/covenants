use cosmwasm_std::Addr;
use covenant_utils::{PolCovenantTerms, CovenantPartiesConfig, LockupConfig};
use cw_storage_plus::Item;

use crate::msg::{ContractState, RagequitConfig};


pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");
pub const PARTIES_CONFIG: Item<CovenantPartiesConfig> = Item::new("parties_config");
pub const LOCKUP_CONFIG: Item<LockupConfig> = Item::new("lockup_config");
pub const RAGEQUIT_CONFIG: Item<RagequitConfig> = Item::new("ragequit_config");
pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");
pub const DEPOSIT_DEADLINE: Item<LockupConfig> = Item::new("deposit_deadline");
pub const COVENANT_TERMS: Item<PolCovenantTerms> = Item::new("covenant_terms");