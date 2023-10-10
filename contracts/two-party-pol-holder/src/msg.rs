use std::fmt;

use astroport::asset::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Attribute, Coin, Decimal, StdError};
use covenant_macros::{clocked, covenant_clock_address, covenant_next_contract};
use covenant_utils::ExpiryConfig;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    pub clock_address: String,
    pub pool_address: String,
    pub next_contract: String,
    pub lockup_config: ExpiryConfig,
    pub ragequit_config: RagequitConfig,
    pub deposit_deadline: Option<ExpiryConfig>,
    pub covenant_config: TwoPartyPolCovenantConfig,
}

impl InstantiateMsg {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        let mut attrs = vec![
            Attribute::new("clock_addr", self.clock_address),
            Attribute::new("pool_address", self.pool_address),
            Attribute::new("next_contract", self.next_contract),
        ];
        attrs.extend(self.ragequit_config.get_response_attributes());
        attrs.extend(self.lockup_config.get_response_attributes());
        attrs
    }
}

#[cw_serde]
pub struct PresetTwoPartyPolHolderFields {
    pub pool_address: String,
    pub lockup_config: ExpiryConfig,
    pub ragequit_config: RagequitConfig,
    pub deposit_deadline: Option<ExpiryConfig>,
    pub party_a: PresetPolParty,
    pub party_b: PresetPolParty,
    pub code_id: u64,
}

#[cw_serde]
pub struct PresetPolParty {
    pub contribution: Coin,
    pub addr: String,
    pub allocation: Decimal,
}

impl PresetTwoPartyPolHolderFields {
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        next_contract: String,
        party_a_router: String,
        party_b_router: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            pool_address: self.pool_address,
            next_contract,
            lockup_config: self.lockup_config,
            ragequit_config: self.ragequit_config,
            deposit_deadline: self.deposit_deadline,
            covenant_config: TwoPartyPolCovenantConfig {
                party_a: TwoPartyPolCovenantParty { 
                    contribution: self.party_a.contribution,
                    addr: self.party_a.addr,
                    allocation: self.party_a.allocation,
                    router: party_a_router,
                },
                party_b: TwoPartyPolCovenantParty { 
                    contribution: self.party_b.contribution,
                    addr: self.party_b.addr,
                    allocation: self.party_b.allocation,
                    router: party_b_router,
                },
            },
        }
    }
}

#[cw_serde]
pub struct TwoPartyPolCovenantConfig {
    pub party_a: TwoPartyPolCovenantParty,
    pub party_b: TwoPartyPolCovenantParty,
}

impl TwoPartyPolCovenantConfig {
    pub fn update_parties(&mut self, p1: TwoPartyPolCovenantParty, p2: TwoPartyPolCovenantParty) {
        if self.party_a.addr == p1.addr {
            self.party_a = p1;
            self.party_b = p2;
        } else {
            self.party_a = p2;
            self.party_b = p1;
        }
    }

    pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
        api.addr_validate(&self.party_a.router)?;
        api.addr_validate(&self.party_b.router)?;
        if self.party_a.allocation + self.party_b.allocation != Decimal::one() {
            return Err(ContractError::AllocationValidationError {});
        }
        Ok(())
    }
}

#[cw_serde]
pub struct TwoPartyPolCovenantParty {
    /// the `denom` and `amount` (`Uint128`) to be contributed by the party
    pub contribution: Coin,
    /// address authorized by the party to perform claims/ragequits
    pub addr: String,
    /// fraction of the entire LP position owned by the party.
    /// upon exiting it becomes 0.00. if counterparty exits, this would
    /// become 1.00, meaning that this party owns the entire position
    /// managed by the covenant.
    pub allocation: Decimal,
    /// address of the interchain router associated with this party
    pub router: String,
}

impl TwoPartyPolCovenantConfig {
    /// if authorized, returns (party, counterparty). otherwise errors
    pub fn authorize_sender(
        &self,
        sender: &Addr,
    ) -> Result<(TwoPartyPolCovenantParty, TwoPartyPolCovenantParty), ContractError> {
        let party_a = self.party_a.clone();
        let party_b = self.party_b.clone();
        if party_a.addr == *sender {
            Ok((party_a, party_b))
        } else if party_b.addr == *sender {
            Ok((party_b, party_a))
        } else {
            Err(ContractError::Unauthorized {})
        }
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
    /// one of the parties have initiated ragequit. the remaining
    /// counterparty with an active position is free to exit at any time.
    Ragequit,
    /// covenant has reached its expiration date.
    Expired,
    /// underlying funds have been withdrawn.
    Complete,
}

impl fmt::Display for ContractState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContractState::Instantiated => write!(f, "instantiated"),
            ContractState::Active => write!(f, "active"),
            ContractState::Ragequit => write!(f, "ragequit"),
            ContractState::Expired => write!(f, "expired"),
            ContractState::Complete => write!(f, "complete"),
        }
    }
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
    #[returns(ExpiryConfig)]
    LockupConfig {},
    #[returns(Addr)]
    PoolAddress {},
    #[returns(TwoPartyPolCovenantParty)]
    ConfigPartyA {},
    #[returns(TwoPartyPolCovenantParty)]
    ConfigPartyB {},
    #[returns(ExpiryConfig)]
    DepositDeadline {},
    #[returns(TwoPartyPolCovenantConfig)]
    Config {},
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
            RagequitConfig::Disabled => vec![Attribute::new("ragequit_config", "disabled")],
            RagequitConfig::Enabled(c) => vec![
                Attribute::new("ragequit_config", "enabled"),
                Attribute::new("ragequit_penalty", c.penalty.to_string()),
            ],
        }
    }

    pub fn validate(
        &self,
        a_allocation: Decimal,
        b_allocation: Decimal,
    ) -> Result<(), ContractError> {
        match self {
            RagequitConfig::Disabled => Ok(()),
            RagequitConfig::Enabled(terms) => {
                // first we validate the range: [0.00, 1.00)
                if terms.penalty >= Decimal::one() || terms.penalty < Decimal::zero() {
                    return Err(ContractError::RagequitPenaltyRangeError {});
                }
                // then validate that rq penalty does not exceed either party allocations
                if terms.penalty > a_allocation || terms.penalty > b_allocation {
                    println!("huh");
                    return Err(ContractError::RagequitPenaltyExceedsPartyAllocationError {});
                }

                Ok(())
            }
        }
    }
}

#[cw_serde]
pub struct RagequitTerms {
    /// decimal based penalty to be applied on a party
    /// for initiating ragequit. Must be in the range of (0.00, 1.00).
    /// Also must not exceed either party allocations in raw values.
    pub penalty: Decimal,
    /// optional rq state. none indicates no ragequit.
    /// some holds the ragequit related config
    pub state: Option<RagequitState>,
}

#[cw_serde]
pub struct RagequitState {
    pub coins: Vec<Coin>,
    pub rq_party: TwoPartyPolCovenantParty,
}

impl RagequitState {
    pub fn from_share_response(
        assets: Vec<Asset>,
        rq_party: TwoPartyPolCovenantParty,
    ) -> Result<RagequitState, StdError> {
        let mut rq_coins: Vec<Coin> = vec![];
        for asset in assets {
            let coin = asset.to_coin()?;
            rq_coins.push(coin);
        }

        Ok(RagequitState {
            coins: rq_coins,
            rq_party,
        })
    }
}
