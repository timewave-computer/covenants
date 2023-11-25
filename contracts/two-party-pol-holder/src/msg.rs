use std::{collections::BTreeMap, fmt};

use astroport::asset::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Attribute, Binary, Coin, CosmosMsg, Decimal, StdError};
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_next_contract,
};
use covenant_utils::{DenomSplit, SplitConfig, SplitType};
use cw_utils::Expiration;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of authorized clock
    pub clock_address: String,
    /// address of the target liquidity pool
    pub pool_address: String,
    /// liquid pooler address
    pub next_contract: String,
    /// config describing the agreed upon duration of POL
    pub lockup_config: Expiration,
    /// config describing early exit dynamics
    pub ragequit_config: RagequitConfig,
    /// deadline for both parties to deposit their funds
    pub deposit_deadline: Expiration,
    /// config describing the covenant dynamics
    pub covenant_config: TwoPartyPolCovenantConfig,
    /// mapping of denoms to their splits
    pub splits: BTreeMap<String, SplitType>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitConfig>,
}

impl InstantiateMsg {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        let fallback_attr = match self.fallback_split.as_ref() {
            Some(split) => split.get_response_attribute("fallback_split".to_string()),
            None => Attribute::new("fallback_split".to_string(), "none".to_string()),
        };
        let splits_attr: Vec<Attribute> = self
            .splits
            .iter()
            .map(|(denom, split_type)| match split_type {
                SplitType::Custom(split_config) => {
                    split_config.get_response_attribute(denom.to_string())
                }
            })
            .collect();

        let mut attrs = vec![
            Attribute::new("clock_addr", self.clock_address.to_string()),
            Attribute::new("pool_address", self.pool_address.to_string()),
            Attribute::new("next_contract", self.next_contract.to_string()),
            Attribute::new("lockup_config", self.lockup_config.to_string()),
            Attribute::new("deposit_deadline", self.deposit_deadline.to_string()),
            fallback_attr,
        ];
        attrs.extend(self.ragequit_config.get_response_attributes());
        attrs.extend(splits_attr);
        attrs
    }
}

#[cw_serde]
pub enum CovenantType {
    Share,
    Side,
}

impl CovenantType {
    pub fn get_response_attribute(&self) -> Attribute {
        Attribute::new(
            "covenant_type",
            match self {
                CovenantType::Share => "share",
                CovenantType::Side => "side",
            },
        )
    }
}

#[cw_serde]
pub struct DenomSplits {
    pub explicit_splits: BTreeMap<String, SplitConfig>,
    pub fallback_split: Option<SplitConfig>,
}

impl DenomSplits {
    pub fn get_fallback_distribution_messages(self, available_coins: Vec<Coin>) -> Vec<CosmosMsg> {
        available_coins
            .iter()
            .filter_map(|c| {
                // explicit splits are distributed via claim/ragequit
                if self.explicit_splits.contains_key(&c.denom) {
                    None
                } else if let Some(fallback_split) = &self.fallback_split {
                    match fallback_split.get_transfer_messages(c.amount, c.denom.to_string(), None)
                    {
                        Ok(msgs) => Some(msgs),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    pub fn get_single_receiver_distribution_messages(
        self,
        available_coins: Vec<Coin>,
        addr: String,
    ) -> Vec<CosmosMsg> {
        available_coins
            .iter()
            .filter_map(|c| {
                // for each coin denom we want to distribute,
                // we look for it in our explicitly defined split configs
                if let Some(config) = self.explicit_splits.get(&c.denom) {
                    // found it, generate the msg or filter out
                    match config.get_transfer_messages(
                        c.amount,
                        c.denom.to_string(),
                        Some(addr.to_string()),
                    ) {
                        Ok(msgs) => Some(msgs),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    pub fn get_shared_distribution_messages(self, available_coins: Vec<Coin>) -> Vec<CosmosMsg> {
        available_coins
            .iter()
            .filter_map(|c| {
                // for each coin denom we want to distribute,
                // we look for it in our explicitly defined split configs
                let split = self.explicit_splits.get(&c.denom);
                if let Some(config) = split {
                    // found it, generate the msg or filter out
                    match config.get_transfer_messages(c.amount, c.denom.to_string(), None) {
                        Ok(msgs) => Some(msgs),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    // todo: clean this up
    pub fn apply_penalty(
        mut self,
        penalty: Decimal,
        party: &TwoPartyPolCovenantParty,
        counterparty: &TwoPartyPolCovenantParty,
    ) -> Result<DenomSplits, ContractError> {
        // we iterate over explicitly defined splits for each denom
        for (denom, mut config) in self.explicit_splits.clone().into_iter() {
            let party_share = config
                .receivers
                // get current party shares or error out if not found
                .get(&party.router)
                .ok_or_else(|| ContractError::PartyNotFound {})?;

            // we do not penalize already null allocations of
            // the ragequitting party
            if party_share > &Decimal::zero() {
                let new_party_share = party_share
                    // add the penalty or return overflow
                    .checked_sub(penalty)
                    .map_err(|e| StdError::overflow(e))?;

                let new_counterparty_share = config
                    .receivers
                    .get(&counterparty.router)
                    .ok_or_else(|| ContractError::PartyNotFound {})?
                    .checked_add(penalty)
                    .map_err(|e| StdError::overflow(e))?;

                // override existing entries with the updated values
                // while keeping the keys
                config
                    .receivers
                    .insert(party.router.to_string(), new_party_share);
                config
                    .receivers
                    .insert(counterparty.router.to_string(), new_counterparty_share);

                // override the existing denom entry with updated config
                self.explicit_splits.insert(denom, config);
            }
        }

        if let Some(mut split_config) = self.fallback_split {
            // apply the ragequit penalty to rq party and its counterparty
            let new_party_share = split_config
                .receivers
                // get current party shares or error out if not found
                .get(party.router.as_str())
                .ok_or_else(|| ContractError::PartyNotFound {})?
                // add the penalty or return overflow
                .checked_sub(penalty)
                .map_err(|e| StdError::overflow(e))?;

            let new_counterparty_share = split_config
                .receivers
                .get(counterparty.router.as_str())
                .ok_or_else(|| ContractError::PartyNotFound {})?
                .checked_add(penalty)
                .map_err(|e| StdError::overflow(e))?;

            // override existing entries with the updated values
            // while keeping the keys
            split_config
                .receivers
                .insert(party.router.to_string(), new_party_share);
            split_config
                .receivers
                .insert(counterparty.router.to_string(), new_counterparty_share);

            // reflect the updated values in self
            self.fallback_split = Some(split_config);
        }

        Ok(self)
    }
}

#[cw_serde]
pub struct PresetTwoPartyPolHolderFields {
    pub pool_address: String,
    pub lockup_config: Expiration,
    pub ragequit_config: RagequitConfig,
    pub deposit_deadline: Expiration,
    pub party_a: PresetPolParty,
    pub party_b: PresetPolParty,
    pub code_id: u64,
    pub label: String,
    pub splits: Vec<DenomSplit>,
    pub fallback_split: Option<SplitConfig>,
    pub covenant_type: CovenantType,
}

#[cw_serde]
pub struct PresetPolParty {
    pub contribution: Coin,
    pub host_addr: String,
    pub controller_addr: String,
    pub allocation: Decimal,
}

impl PresetTwoPartyPolHolderFields {
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        next_contract: String,
        party_a_router: &str,
        party_b_router: &str,
    ) -> Result<InstantiateMsg, StdError> {
        let mut remapped_splits: BTreeMap<String, SplitType> = BTreeMap::new();
        for denom_split in &self.splits {
            match &denom_split.split {
                SplitType::Custom(config) => {
                    let remapped_split = config.remap_receivers_to_routers(
                        self.party_a.controller_addr.to_string(),
                        party_a_router.to_string(),
                        self.party_b.controller_addr.to_string(),
                        party_b_router.to_string(),
                    )?;
                    remapped_splits.insert(
                        denom_split.denom.to_string(),
                        SplitType::Custom(remapped_split),
                    );
                }
            }
        }

        let remapped_fallback = match &self.fallback_split {
            Some(split_config) => Some(split_config.remap_receivers_to_routers(
                self.party_a.controller_addr.to_string(),
                party_a_router.to_string(),
                self.party_b.controller_addr.to_string(),
                party_b_router.to_string(),
            )?),
            None => None,
        };

        Ok(InstantiateMsg {
            clock_address,
            pool_address: self.pool_address,
            next_contract,
            lockup_config: self.lockup_config,
            ragequit_config: self.ragequit_config,
            deposit_deadline: self.deposit_deadline,
            covenant_config: TwoPartyPolCovenantConfig {
                party_a: TwoPartyPolCovenantParty {
                    contribution: self.party_a.contribution,
                    allocation: self.party_a.allocation,
                    router: party_a_router.to_string(),
                    host_addr: self.party_a.host_addr,
                    controller_addr: self.party_a.controller_addr,
                },
                party_b: TwoPartyPolCovenantParty {
                    contribution: self.party_b.contribution,
                    allocation: self.party_b.allocation,
                    router: party_b_router.to_string(),
                    host_addr: self.party_b.host_addr,
                    controller_addr: self.party_b.controller_addr,
                },
                covenant_type: self.covenant_type,
            },
            splits: remapped_splits,
            fallback_split: remapped_fallback,
        })
    }
}

#[cw_serde]
pub struct TwoPartyPolCovenantConfig {
    pub party_a: TwoPartyPolCovenantParty,
    pub party_b: TwoPartyPolCovenantParty,
    pub covenant_type: CovenantType,
}

impl TwoPartyPolCovenantConfig {
    pub fn update_parties(&mut self, p1: TwoPartyPolCovenantParty, p2: TwoPartyPolCovenantParty) {
        if self.party_a.controller_addr == p1.controller_addr {
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

    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        let mut attributes = vec![];
        let party_a_attributes: Vec<Attribute> = self.party_a.get_response_attributes();
        let party_b_attributes: Vec<Attribute> = self.party_b.get_response_attributes();
        attributes.extend(party_a_attributes);
        attributes.extend(party_b_attributes);
        attributes.push(self.covenant_type.get_response_attribute());
        attributes
    }
}

#[cw_serde]
pub struct TwoPartyPolCovenantParty {
    /// the `denom` and `amount` (`Uint128`) to be contributed by the party
    pub contribution: Coin,
    /// neutron address authorized by the party to perform claims/ragequits
    pub host_addr: String,
    /// address of the party on the controller chain (final receiver)
    pub controller_addr: String,
    /// fraction of the entire LP position owned by the party.
    /// upon exiting it becomes 0.00. if counterparty exits, this would
    /// become 1.00, meaning that this party owns the entire position
    /// managed by the covenant.
    pub allocation: Decimal,
    /// address of the interchain router associated with this party
    pub router: String,
}

impl TwoPartyPolCovenantParty {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new("contribution", self.contribution.to_string()),
            Attribute::new("host_addr", self.host_addr.to_string()),
            Attribute::new("controller_addr", self.controller_addr.to_string()),
            Attribute::new("allocation", self.allocation.to_string()),
            Attribute::new("router", self.router.to_string()),
        ]
    }
}

impl TwoPartyPolCovenantConfig {
    /// if authorized, returns (party, counterparty). otherwise errors
    pub fn authorize_sender(
        &self,
        sender: String,
    ) -> Result<(TwoPartyPolCovenantParty, TwoPartyPolCovenantParty), ContractError> {
        let party_a = self.party_a.clone();
        let party_b = self.party_b.clone();
        if party_a.host_addr == sender {
            Ok((party_a, party_b))
        } else if party_b.host_addr == sender {
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
    /// distribute any unspecified denoms
    DistributeFallbackSplit {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        next_contract: Option<String>,
        lockup_config: Option<Expiration>,
        deposit_deadline: Option<Expiration>,
        pool_address: Option<String>,
        ragequit_config: Option<RagequitConfig>,
        covenant_config: Option<TwoPartyPolCovenantConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
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

impl ContractState {
    pub fn validate_claim_state(&self) -> Result<(), ContractError> {
        match self {
            ContractState::Ragequit => Ok(()),
            ContractState::Expired => Ok(()),
            _ => return Err(ContractError::ClaimError {}),
        }
    }
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
#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(RagequitConfig)]
    RagequitConfig {},
    #[returns(Expiration)]
    LockupConfig {},
    #[returns(Addr)]
    PoolAddress {},
    #[returns(TwoPartyPolCovenantParty)]
    ConfigPartyA {},
    #[returns(TwoPartyPolCovenantParty)]
    ConfigPartyB {},
    #[returns(Expiration)]
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
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
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
