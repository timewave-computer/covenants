use cw_utils::Expiration;

use crate::setup::suite_builder::SuiteBuilder;


pub struct SinglePartyHolderInstantiate {
    pub msg: covenant_single_party_pol_holder::msg::InstantiateMsg,
}

impl From<SinglePartyHolderInstantiate> for covenant_single_party_pol_holder::msg::InstantiateMsg {
    fn from(value: SinglePartyHolderInstantiate) -> Self {
        value.msg
    }
}

impl SinglePartyHolderInstantiate {
    pub fn new(
        withdrawer: Option<String>,
        withdraw_to: Option<String>,
        emergency_committee_addr: Option<String>,
        pooler_address: String,
        lockup_period: Expiration,
    ) -> Self {
        Self {
            msg: covenant_single_party_pol_holder::msg::InstantiateMsg {
                withdrawer,
                withdraw_to,
                emergency_committee_addr,
                pooler_address,
                lockup_period,
            }
        }
    }

    pub fn with_withdrawer(&mut self, addr: &str) -> &mut Self {
        self.msg.withdrawer = Some(addr.to_string());
        self
    }

    pub fn with_withdraw_to(&mut self, addr: &str) -> &mut Self {
        self.msg.withdraw_to = Some(addr.to_string());
        self
    }

    pub fn with_emergency_committee_addr(&mut self, addr: &str) -> &mut Self {
        self.msg.emergency_committee_addr = Some(addr.to_string());
        self
    }

    pub fn with_pooler_address(&mut self, addr: &str) -> &mut Self {
        self.msg.pooler_address = addr.to_string();
        self
    }

    pub fn with_lockup_period(&mut self, period: Expiration) -> &mut Self {
        self.msg.lockup_period = period;
        self
    }
}

impl SinglePartyHolderInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        pooler_address: String,
        lockup_period: Expiration,
        withdrawer: Option<String>,
        withdraw_to: Option<String>,
        emergency_committee_addr: Option<String>,
    ) -> Self {
        Self::new(
            withdrawer,
            withdraw_to,
            emergency_committee_addr,
            pooler_address,
            lockup_period,
        )
    }
}
