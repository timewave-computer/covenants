use localic_std::{modules::cosmwasm::contract_query, transactions::ChainRequestBuilder};

pub enum Covenant<'a> {
    TwoPartyPol {
        rb: &'a ChainRequestBuilder,
        contract_address: &'a str,
    },
}

impl<'a> Covenant<'a> {
    fn get_rb(&self) -> &ChainRequestBuilder {
        match self {
            Covenant::TwoPartyPol { rb, .. } => rb,
        }
    }

    fn get_contract_address(&self) -> &str {
        match self {
            Covenant::TwoPartyPol {
                contract_address, ..
            } => contract_address,
        }
    }

    fn query(&self, query_msg: &str) -> String {
        let query_response = contract_query(self.get_rb(), self.get_contract_address(), query_msg);
        query_response["data"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    pub fn query_clock_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::ClockAddress {}
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }

    pub fn query_holder_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::HolderAddress {}
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }

    pub fn query_liquid_pooler_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::LiquidPoolerAddress {}
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }

    pub fn query_interchain_router_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::InterchainRouterAddress { party }
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }

    pub fn query_ibc_forwarder_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::IbcForwarderAddress { party }
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }

    pub fn query_deposit_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => {
                &valence_covenant_two_party_pol::msg::QueryMsg::PartyDepositAddress { party }
            }
        };

        self.query(&serde_json::to_string(query_msg).unwrap())
    }
}
