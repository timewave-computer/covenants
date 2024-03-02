use covenant_utils::{CovenantPartiesConfig, CovenantTerms};
use cw_utils::Expiration;

use crate::setup::suite_builder::SuiteBuilder;


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
            }
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
        builder: &SuiteBuilder,
        clock_address: String,
        next_contract: String,
        lockup_config: Expiration,
        covenant_terms: CovenantTerms,
        parties_config: CovenantPartiesConfig,
    ) -> Self {
        Self::new(
            clock_address,
            next_contract,
            lockup_config,
            covenant_terms,
            parties_config,
        )
    }
}
