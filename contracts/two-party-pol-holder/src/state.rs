use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_utils::Expiration;

use crate::msg::{ContractState, DenomSplits, RagequitConfig, TwoPartyPolCovenantConfig};

pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// authorized clock contract
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

/// the LP module that we send the deposited funds to
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

/// address of the liquidity pool to which we provide liquidity
pub const POOLER_ADDRESS: Item<Addr> = Item::new("pooler_address");

/// configuration describing the lockup period after which parties are
/// no longer subject to ragequit penalties in order to exit their position
pub const LOCKUP_CONFIG: Item<Expiration> = Item::new("lockup_config");

/// configuration describing the deposit period during which parties
/// are expected to fulfill their parts of the covenant
pub const DEPOSIT_DEADLINE: Item<Expiration> = Item::new("deposit_deadline");

/// configuration describing the penalty applied to the allocation
/// of the party initiating the ragequit
pub const RAGEQUIT_CONFIG: Item<RagequitConfig> = Item::new("ragequit_config");

/// configuration storing both parties information
pub const COVENANT_CONFIG: Item<TwoPartyPolCovenantConfig> = Item::new("covenant_config");

/// stores the configuration describing how to distribute every denom
pub const DENOM_SPLITS: Item<DenomSplits> = Item::new("denom_splits");

pub const WITHDRAW_STATE: Item<WithdrawState> = Item::new("withdraw_state");

#[cw_serde]
pub enum WithdrawState {
    Processing { claimer_addr: String },
    Emergency {},
}
