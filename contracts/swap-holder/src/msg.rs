use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Timestamp, Addr, Attribute, BlockInfo, Uint128};
use covenant_macros::clocked;

use crate::error::ContractError;


#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// address of the next contract to forward the funds to.
    /// usually expected tobe the splitter.
    pub next_contract: String,
    /// block height of covenant expiration. Position is exited 
    /// automatically upon reaching that height.
    pub lockup_config: LockupConfig,
    /// parties engaged in the POL.
    pub parties_config: PartiesConfig,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}


#[cw_serde]
pub enum ContractState {
    Instantiated,
    /// covenant has reached its expiration date.
    Expired,
    /// underlying funds have been withdrawn.
    Complete,
}

#[cw_serde]
pub struct PartiesConfig {
    pub party_a: Party,
    pub party_b: Party,
}

#[cw_serde]
pub struct Party {
    /// authorized address of the party
    pub addr: Addr,
    /// denom provided by the party
    pub provided_denom: String,
    /// amount of the denom above to be expected
    pub amount: Uint128,
}

/// enum based configuration of the lockup period.
#[cw_serde]
pub enum LockupConfig {
    /// no lockup configured
    None,
    /// block height based lockup config
    Block(u64),
    /// timestamp based lockup config
    Time(Timestamp),
}


impl LockupConfig {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        match self {
            LockupConfig::None => vec![
                Attribute::new("lockup_config", "none"),
            ],
            LockupConfig::Block(h) => vec![
                Attribute::new("lockup_config_expiry_block_height", h.to_string()),
            ],
            LockupConfig::Time(t) => vec![
                Attribute::new("lockup_config_expiry_block_timestamp", t.to_string()),
            ],
        }
    }

    /// validates that the lockup config being stored is not already expired.
    pub fn validate(&self, block_info: BlockInfo) -> Result<&LockupConfig, ContractError> {
        match self {
            LockupConfig::None => Ok(self),
            LockupConfig::Block(h) => {
                if h > &block_info.height {
                    Ok(self)
                } else {
                    Err(ContractError::Std(cosmwasm_std::StdError::GenericErr { msg: "invalid".to_string() }))
                }
            },
            LockupConfig::Time(t) => {
                if t.nanos() > block_info.time.nanos() {
                    Ok(self)
                } else {
                    Err(ContractError::Std(cosmwasm_std::StdError::GenericErr { msg: "invalid".to_string() }))
                }
            },
        }
    }

    /// compares current block info with the stored lockup config.
    /// returns false if no lockup configuration is stored.
    /// otherwise, returns true if the current block is past the stored info.
    pub fn is_due(self, block_info: BlockInfo) -> bool {
        match self {
            LockupConfig::None => false, // or.. true? should not be called
            LockupConfig::Block(h) => h < block_info.height,
            LockupConfig::Time(t) => t.nanos() < block_info.time.nanos(),
        }
    }
}