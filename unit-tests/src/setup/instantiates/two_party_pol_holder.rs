use std::collections::BTreeMap;

use covenant_utils::split::SplitConfig;
use cw_utils::Expiration;

use crate::setup::suite_builder::SuiteBuilder;

#[derive(Clone)]
pub struct TwoPartyHolderInstantiate {
    pub msg: covenant_two_party_pol_holder::msg::InstantiateMsg,
}

impl From<TwoPartyHolderInstantiate> for covenant_two_party_pol_holder::msg::InstantiateMsg {
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
        ragequit_config: covenant_two_party_pol_holder::msg::RagequitConfig,
        deposit_deadline: Expiration,
        covenant_config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
        splits: BTreeMap<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        Self {
            msg: covenant_two_party_pol_holder::msg::InstantiateMsg {
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

    pub fn with_ragequit_config(&mut self, config: covenant_two_party_pol_holder::msg::RagequitConfig) -> &mut Self {
        self.msg.ragequit_config = config;
        self
    }

    pub fn with_deposit_deadline(&mut self, config: Expiration) -> &mut Self {
        self.msg.deposit_deadline = config;
        self
    }

    pub fn with_covenant_config(&mut self, config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig) -> &mut Self {
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
        builder: &SuiteBuilder,
        clock_address: String,
        next_contract: String,
        lockup_config: Expiration,
        ragequit_config: covenant_two_party_pol_holder::msg::RagequitConfig,
        deposit_deadline: Expiration,
        covenant_config: covenant_two_party_pol_holder::msg::TwoPartyPolCovenantConfig,
        splits: BTreeMap<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        Self::new(
            clock_address,
            next_contract,
            lockup_config,
            ragequit_config,
            deposit_deadline,
            covenant_config,
            splits,
            fallback_split,
            emergency_committee_addr,
        )
    }
}