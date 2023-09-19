use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp, BlockInfo, Attribute};
use covenant_macros::{clocked, covenant_clock_address, covenant_next_contract};
use covenant_utils::PolCovenantTerms;

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
    /// deadline for both parties to deposit the funds
    /// TODO: rename LockupConfig to something more generic
    /// to represent block/time based expiration
    pub deposit_deadline: Option<LockupConfig>,
    pub covenant_terms: PolCovenantTerms,
}

impl InstantiateMsg {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        let mut attrs = vec![
            Attribute::new("clock_addr", self.clock_address),
            Attribute::new("pool_address", self.pool_address),
            Attribute::new("next_contract", self.next_contract),
        ];
        attrs.extend(self.parties_config.get_response_attributes());
        attrs.extend(self.ragequit_config.get_response_attributes());
        attrs.extend(self.lockup_config.get_response_attributes());
        attrs
    }
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
    /// contract is instantiated and awaiting for deposits from
    /// both parties involved
    Instantiated,
    /// funds have been forwarded to the LP module. from the perspective
    /// of this contract that indicates an active LP position.
    /// TODO: think about whether this is a fair assumption to make.
    Active,
    /// one of the parties have initiated ragequit.
    /// party with an active position is free to exit at any time.
    Ragequit,
    /// covenant has reached its expiration date.
    Expired,
    /// underlying funds have been withdrawn.
    Complete,
}

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
    pub fn validate_config(&self) -> Result<&PartiesConfig, ContractError> {
        if self.party_a.share + self.party_b.share == Decimal::one() {
            Ok(self)
        } else {
            Err(ContractError::InvolvedPartiesConfigError {})
        }
    }

    /// validates the caller and returns an error if caller is unauthorized,
    /// or the calling party if its authorized
    pub fn validate_caller(&self, caller: Addr) -> Result<Party, ContractError> {
        let a = self.clone().party_a;
        let b = self.clone().party_b;
        if a.addr == caller {
            Ok(a)
        } else if b.addr == caller {
            Ok(b)
        } else {
            Err(ContractError::RagequitUnauthorized {})
        }
    }

    /// subtracts the ragequit penalty to the ragequitting party
    /// and adds it to the other party
    pub fn apply_ragequit_penalty(
        mut self,
        rq_party: Party,
        penalty: Decimal
    ) -> Result<PartiesConfig, ContractError> {
        if rq_party.addr == self.party_a.addr {
            self.party_a.share -= penalty;
            self.party_b.share += penalty;
        } else {
            self.party_a.share += penalty;
            self.party_b.share -= penalty;
        }
        Ok(self)
    }

    pub fn get_party_by_addr(self, addr: Addr) -> Result<Party, ContractError> {
        if self.party_a.addr == addr {
            Ok(self.party_a)
        } else if self.party_b.addr == addr {
            Ok(self.party_b)
        } else {
            Err(ContractError::PartyNotFound {})
        }
    }
}

impl PartiesConfig {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        vec![
            Attribute::new("party_a_address", self.party_a.addr),
            Attribute::new("party_a_share", self.party_a.share.to_string()),
            Attribute::new("party_a_provided_denom", self.party_a.provided_denom),
            Attribute::new("party_a_active_position", self.party_a.active_position.to_string()),
            Attribute::new("party_b_address", self.party_b.addr),
            Attribute::new("party_b_share", self.party_b.share.to_string()),
            Attribute::new("party_b_provided_denom", self.party_b.provided_denom),
            Attribute::new("party_b_active_position", self.party_b.active_position.to_string()),
        ]
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

impl RagequitConfig {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        match self {
            RagequitConfig::Disabled => vec![
                Attribute::new("ragequit_config", "disabled"),
            ],
            RagequitConfig::Enabled(c) => vec![
                Attribute::new("ragequit_config", "enabled"),
                Attribute::new("ragequit_penalty", c.penalty.to_string()),
                Attribute::new("ragequit_active", c.active.to_string()),
            ],
        }
    }
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
    pub fn validate(&self, block_info: &BlockInfo) -> Result<&LockupConfig, ContractError> {
        match self {
            LockupConfig::None => Ok(self),
            LockupConfig::Block(h) => {
                if h > &block_info.height {
                    Ok(self)
                } else {
                    Err(ContractError::LockupValidationError {})
                }
            },
            LockupConfig::Time(t) => {
                if t.nanos() > block_info.time.nanos() {
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
            LockupConfig::None => false, // or.. true? should not be called
            LockupConfig::Block(h) => h < block_info.height,
            LockupConfig::Time(t) => t.nanos() < block_info.time.nanos(),
        }
    }
}
