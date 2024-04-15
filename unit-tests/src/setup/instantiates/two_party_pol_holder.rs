use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal};
use covenant_utils::split::SplitConfig;
use cw_utils::Expiration;

use crate::setup::{DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

#[derive(Clone)]
pub struct TwoPartyHolderInstantiate {
    pub msg: valence_two_party_pol_holder::msg::InstantiateMsg,
}

impl From<TwoPartyHolderInstantiate> for valence_two_party_pol_holder::msg::InstantiateMsg {
    fn from(value: TwoPartyHolderInstantiate) -> Self {
        value.msg
    }
}

impl TwoPartyHolderInstantiate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        clock_address: String,
        next_contract: String,
        lockup_config: Expiration,
        ragequit_config: valence_two_party_pol_holder::msg::RagequitConfig,
        deposit_deadline: Expiration,
        covenant_config: valence_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
        splits: BTreeMap<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        Self {
            msg: valence_two_party_pol_holder::msg::InstantiateMsg {
                clock_address,
                next_contract,
                lockup_config,
                ragequit_config,
                deposit_deadline,
                covenant_config,
                splits,
                fallback_split,
                emergency_committee_addr,
            },
        }
    }

    /* Change functions */
    pub fn with_clock(&mut self, addr: &str) -> &mut Self {
        self.msg.clock_address = addr.to_string();
        self
    }

    pub fn with_next_contract(&mut self, addr: &str) -> &mut Self {
        self.msg.next_contract = addr.to_string();
        self
    }

    pub fn with_lockup_config(&mut self, config: Expiration) -> &mut Self {
        self.msg.lockup_config = config;
        self
    }

    pub fn with_ragequit_config(
        &mut self,
        config: valence_two_party_pol_holder::msg::RagequitConfig,
    ) -> &mut Self {
        self.msg.ragequit_config = config;
        self
    }

    pub fn with_deposit_deadline(&mut self, config: Expiration) -> &mut Self {
        self.msg.deposit_deadline = config;
        self
    }

    pub fn with_covenant_config(
        &mut self,
        config: valence_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
    ) -> &mut Self {
        self.msg.covenant_config = config;
        self
    }

    pub fn with_splits(&mut self, splits: BTreeMap<String, SplitConfig>) -> &mut Self {
        self.msg.splits = splits;
        self
    }

    pub fn with_fallback_split(&mut self, split: SplitConfig) -> &mut Self {
        self.msg.fallback_split = Some(split);
        self
    }

    pub fn with_emergency_committee(&mut self, addr: &str) -> &mut Self {
        self.msg.emergency_committee_addr = Some(addr.to_string());
        self
    }
}

impl TwoPartyHolderInstantiate {
    pub fn default(
        clock_address: String,
        next_contract: String,
        party_a_addr: Addr,
        party_b_addr: Addr,
    ) -> Self {
        let mut splits = BTreeMap::new();
        splits.insert(party_a_addr.to_string(), Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_addr.to_string(), Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        Self {
            msg: valence_two_party_pol_holder::msg::InstantiateMsg {
                clock_address,
                next_contract,
                lockup_config: Expiration::AtHeight(200000),
                ragequit_config: valence_two_party_pol_holder::msg::RagequitConfig::Disabled {},
                deposit_deadline: Expiration::AtHeight(100000),
                covenant_config: valence_two_party_pol_holder::msg::TwoPartyPolCovenantConfig {
                    party_a: valence_two_party_pol_holder::msg::TwoPartyPolCovenantParty {
                        contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                        host_addr: party_a_addr.to_string(),
                        controller_addr: party_a_addr.to_string(),
                        allocation: Decimal::from_str("0.5").unwrap(),
                        router: party_a_addr.to_string(),
                    },
                    party_b: valence_two_party_pol_holder::msg::TwoPartyPolCovenantParty {
                        contribution: coin(10_000, DENOM_LS_ATOM_ON_NTRN),
                        host_addr: party_b_addr.to_string(),
                        controller_addr: party_b_addr.to_string(),
                        allocation: Decimal::from_str("0.5").unwrap(),
                        router: party_b_addr.to_string(),
                    },
                    covenant_type: valence_two_party_pol_holder::msg::CovenantType::Share {},
                },
                splits: denom_to_split_config_map,
                fallback_split: None,
                emergency_committee_addr: None,
            },
        }
    }
}
