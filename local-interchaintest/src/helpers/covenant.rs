use localic_std::{modules::cosmwasm::contract_query, transactions::ChainRequestBuilder};

pub enum Covenant<'a> {
    TwoPartyPol {
        rb: &'a ChainRequestBuilder,
        contract_address: &'a str,
    },
    SinglePartyPol {
        rb: &'a ChainRequestBuilder,
        contract_address: &'a str,
    },
}

impl<'a> Covenant<'a> {
    fn get_rb(&self) -> &ChainRequestBuilder {
        match self {
            Covenant::TwoPartyPol { rb, .. } => rb,
            Covenant::SinglePartyPol { rb, .. } => rb,
        }
    }

    fn get_contract_address(&self) -> &str {
        match self {
            Covenant::TwoPartyPol {
                contract_address, ..
            } => contract_address,
            Covenant::SinglePartyPol {
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
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::ClockAddress {},
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::ClockAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_holder_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::HolderAddress {},
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::HolderAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_liquid_pooler_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::LiquidPoolerAddress {},
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::LiquidPoolerAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_liquid_staker_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => return String::new(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::LiquidStakerAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_splitter_address(&self) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => return String::new(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::SplitterAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_interchain_router_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::InterchainRouterAddress { party },
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::InterchainRouterAddress {},
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_ibc_forwarder_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::IbcForwarderAddress { party },
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::IbcForwarderAddress {
                    ty: party,
                },
            )
            .unwrap(),
        };

        self.query(query_msg)
    }

    pub fn query_deposit_address(&self, party: String) -> String {
        let query_msg = match self {
            Covenant::TwoPartyPol { .. } => &serde_json::to_string(
                &valence_covenant_two_party_pol::msg::QueryMsg::PartyDepositAddress { party },
            )
            .unwrap(),
            Covenant::SinglePartyPol { .. } => &serde_json::to_string(
                &valence_covenant_single_party_pol::msg::QueryMsg::PartyDepositAddress {},
            )
            .unwrap(),
        };
        self.query(query_msg)
    }
}
