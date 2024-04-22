use cw_utils::Expiration;

pub struct SinglePartyHolderInstantiate {
    pub msg: valence_single_party_pol_holder::msg::InstantiateMsg,
}

impl From<SinglePartyHolderInstantiate> for valence_single_party_pol_holder::msg::InstantiateMsg {
    fn from(value: SinglePartyHolderInstantiate) -> Self {
        value.msg
    }
}

impl SinglePartyHolderInstantiate {
    pub fn new(
        withdrawer: String,
        withdraw_to: String,
        emergency_committee_addr: Option<String>,
        pooler_address: String,
        lockup_period: Expiration,
    ) -> Self {
        Self {
            msg: valence_single_party_pol_holder::msg::InstantiateMsg {
                withdrawer,
                withdraw_to,
                emergency_committee_addr,
                pooler_address,
                lockup_period,
            },
        }
    }

    pub fn with_withdrawer(&mut self, addr: String) -> &mut Self {
        self.msg.withdrawer = addr;
        self
    }

    pub fn with_withdraw_to(&mut self, addr: String) -> &mut Self {
        self.msg.withdraw_to = addr;
        self
    }

    pub fn with_emergency_committee_addr(&mut self, addr: Option<String>) -> &mut Self {
        self.msg.emergency_committee_addr = addr;
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
    pub fn default(pooler_address: String) -> Self {
        Self {
            msg: valence_single_party_pol_holder::msg::InstantiateMsg {
                withdrawer: pooler_address.to_string(),
                withdraw_to: pooler_address.to_string(),
                emergency_committee_addr: Some(pooler_address.to_string()),
                pooler_address,
                lockup_period: Expiration::AtHeight(100000),
            },
        }
    }
}
