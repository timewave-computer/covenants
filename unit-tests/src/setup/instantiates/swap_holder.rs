use cosmwasm_std::{Addr, Uint128};
use covenant_utils::{CovenantPartiesConfig, CovenantParty, CovenantTerms, ReceiverConfig};
use cw_utils::Expiration;

use crate::setup::{DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

pub struct SwapHolderInstantiate {
    pub msg: covenant_swap_holder::msg::InstantiateMsg,
}

impl From<SwapHolderInstantiate> for covenant_swap_holder::msg::InstantiateMsg {
    fn from(value: SwapHolderInstantiate) -> Self {
        value.msg
    }
}

impl SwapHolderInstantiate {
    pub fn new(
        clock_address: String,
        next_contract: String,
        lockup_config: Expiration,
        covenant_terms: CovenantTerms,
        parties_config: CovenantPartiesConfig,
    ) -> Self {
        Self {
            msg: covenant_swap_holder::msg::InstantiateMsg {
                clock_address,
                next_contract,
                lockup_config,
                covenant_terms,
                parties_config,
            },
        }
    }

    pub fn with_clock_address(&mut self, addr: &str) -> &mut Self {
        self.msg.clock_address = addr.to_string();
        self
    }

    pub fn with_next_contract(&mut self, addr: &str) -> &mut Self {
        self.msg.next_contract = addr.to_string();
        self
    }

    pub fn with_lockup_config(&mut self, period: Expiration) -> &mut Self {
        self.msg.lockup_config = period;
        self
    }

    pub fn with_covenant_terms(&mut self, terms: CovenantTerms) -> &mut Self {
        self.msg.covenant_terms = terms;
        self
    }

    pub fn with_parties_config(&mut self, config: CovenantPartiesConfig) -> &mut Self {
        self.msg.parties_config = config;
        self
    }
}

impl SwapHolderInstantiate {
    pub fn default(
        clock_address: String,
        next_contract: String,
        party_a_addr: Addr,
        party_b_addr: Addr,
    ) -> Self {
        Self {
            msg: covenant_swap_holder::msg::InstantiateMsg {
                clock_address,
                next_contract,
                lockup_config: Expiration::AtHeight(1000000),
                covenant_terms: CovenantTerms::TokenSwap(covenant_utils::SwapCovenantTerms {
                    party_a_amount: Uint128::new(100000),
                    party_b_amount: Uint128::new(100000),
                }),
                parties_config: CovenantPartiesConfig {
                    party_a: CovenantParty {
                        addr: party_a_addr.to_string(),
                        native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                        receiver_config: ReceiverConfig::Native(party_a_addr.to_string()),
                    },
                    party_b: CovenantParty {
                        addr: party_b_addr.to_string(),
                        native_denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                        receiver_config: ReceiverConfig::Native(party_b_addr.to_string()),
                    },
                },
            },
        }
    }
}
