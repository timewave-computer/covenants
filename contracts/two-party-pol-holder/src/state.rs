use cosmwasm_std::Addr;
use covenant_utils::ExpiryConfig;
use cw_storage_plus::Item;

use crate::msg::{ContractState, RagequitConfig, TwoPartyPolCovenantConfig};

pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// authorized clock contract
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

/// the LP module that we send the deposited funds to
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

/// configuration describing the lockup period after which parties are
/// no longer subject to ragequit penalties in order to exit their position
pub const LOCKUP_CONFIG: Item<ExpiryConfig> = Item::new("lockup_config");

/// configuration describing the deposit period during which parties
/// are expected to fulfill their parts of the covenant
pub const DEPOSIT_DEADLINE: Item<ExpiryConfig> = Item::new("deposit_deadline");

/// configuration describing the penalty applied to the allocation
/// of the party initiating the ragequit
pub const RAGEQUIT_CONFIG: Item<RagequitConfig> = Item::new("ragequit_config");

/// address of the liquidity pool to which we provide liquidity
pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");

/// address of the cw20 token issued for providing liquidity to the pool
pub const LP_TOKEN: Item<Addr> = Item::new("lp_token");

/// configuration storing both parties information
pub const COVENANT_CONFIG: Item<TwoPartyPolCovenantConfig> = Item::new("covenant_config");
