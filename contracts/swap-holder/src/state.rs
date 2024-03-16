use cosmwasm_std::Addr;
use covenant_utils::{CovenantPartiesConfig, CovenantTerms};
use cw_storage_plus::Item;
use cw_utils::Expiration;

use crate::msg::{ContractState, RefundConfig};

pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");
pub const PARTIES_CONFIG: Item<CovenantPartiesConfig> = Item::new("parties_config");
pub const LOCKUP_CONFIG: Item<Expiration> = Item::new("lockup_config");
pub const COVENANT_TERMS: Item<CovenantTerms> = Item::new("covenant_terms");
pub const REFUND_CONFIG: Item<RefundConfig> = Item::new("refund_config");
