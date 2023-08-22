use std::cmp::Ordering;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp, BlockInfo};
use covenant_macros::{clocked, covenant_deposit_address, covenant_clock_address, covenant_next_contract};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// address of the pool
    pub pool_address: String,
    /// address of the next contract to forward the funds to.
    /// usually expected tobe the splitter.
    pub next_contract: String,
    /// block height of covenant expiration. Position is exited 
    /// automatically upon reaching that height.
    pub lockup_config: LockupConfig,
    /// configuration for ragequit
    pub ragequit_config: RagequitConfig,
    /// parties engaged in the POL.
    pub parties_config: PartiesConfig,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    /// initiate the ragequit
    Ragequit {},
    /// withdraw the liquidity party is entitled to
    Claim {},
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    /// one of the parties have initiated ragequit.
    /// party with an active position is free to exit at any time.
    Ragequit,
    /// covenant has reached its expiration date.
    ExpirationReached,
    /// underlying funds have been withdrawn.
    Complete,
}

#[covenant_deposit_address]
#[covenant_clock_address]
#[covenant_next_contract]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(RagequitConfig)]
    RagequitConfig {},
    #[returns(LockupConfig)]
    LockupConfig {},
    #[returns(PartiesConfig)]
    PartiesConfig {},
}

#[cw_serde]
pub struct PartiesConfig {
    pub party_a: Party,
    pub party_b: Party,
}

impl PartiesConfig {
    /// validates the decimal shares of parties involved
    /// that must add up to 1.0
    pub fn validate(self) -> Result<PartiesConfig, ContractError> {
        if self.party_a.share + self.party_b.share == Decimal::one() {
            Ok(self)
        } else {
            Err(ContractError::InvolvedPartiesConfigError {})
        }
    }
}

#[cw_serde]
pub struct Party {
    /// authorized address of the party
    pub addr: Addr,
    /// decimal share of the LP position (e.g. 1/2)
    pub share: Decimal,
    /// denom provided by the party
    pub provided_denom: String,
    /// whether party is actively providing liquidity
    pub active_position: bool,
}

#[cw_serde]
pub enum RagequitConfig {
    /// ragequit is disabled
    Disabled,
    /// ragequit is enabled with `RagequitTerms`
    Enabled(RagequitTerms),
}

#[cw_serde]
pub struct RagequitTerms {
    /// decimal based penalty to be applied on a party
    /// for initiating ragequit. this fraction is then
    /// added to the counterparty that did not initiate
    /// the ragequit
    pub penalty: Decimal,
    /// bool flag to indicate whether ragequit had been
    /// initiated
    pub active: bool,
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
    /// validates that the lockup config being stored is not already expired.
    pub fn validate(self, block_info: BlockInfo) -> Result<LockupConfig, ContractError> {
        match self {
            LockupConfig::None => Ok(self),
            LockupConfig::Block(h) => {
                if h > block_info.height {
                    Ok(self)
                } else {
                    Err(ContractError::LockupValidationError {})
                }
            },
            LockupConfig::Time(t) => {
                if t.cmp(&block_info.time) != Ordering::Less {
                    Ok(self)
                } else {
                    Err(ContractError::LockupValidationError {})
                }
            },
        }
    }

    /// compares current block info with the stored lockup config.
    /// returns false if no lockup configuration is stored.
    /// otherwise, returns true if the current block is past the stored info.
    pub fn is_due(self, block_info: BlockInfo) -> bool {
        match self {
            LockupConfig::None => false, // or.. true?
            LockupConfig::Block(b) => block_info.height >= b,
            LockupConfig::Time(t) => t.nanos() < block_info.time.nanos(),
        }
    }
}
