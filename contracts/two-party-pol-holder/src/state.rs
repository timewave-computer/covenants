use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use covenant_utils::op_mode::ContractOperationMode;
use cw_storage_plus::Item;
use cw_utils::Expiration;

use crate::msg::{
    ContractState, DenomSplits, RagequitConfig, RagequitTerms, TwoPartyPolCovenantConfig,
};

pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

pub const CONTRACT_OP_MODE: Item<ContractOperationMode> = Item::new("contract_op_mode");

/// address of the liquidity pool to which we provide liquidity
pub const LIQUID_POOLER_ADDRESS: Item<Addr> = Item::new("pooler_address");

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
    Processing {
        claimer_addr: String,
    },
    ProcessingRagequit {
        claimer_addr: String,
        terms: RagequitTerms,
    },
    Emergency {},
}
