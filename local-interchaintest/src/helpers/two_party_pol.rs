use crate::utils::constants::EXECUTE_FLAGS;
use localic_std::{
    modules::cosmwasm::{contract_execute, contract_query},
    transactions::ChainRequestBuilder,
};

use super::common::Messages;

pub fn query_clock_address(rb: &ChainRequestBuilder, contract_address: &str) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(&valence_covenant_two_party_pol::msg::QueryMsg::ClockAddress {})
            .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_holder_address(rb: &ChainRequestBuilder, contract_address: &str) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(&valence_covenant_two_party_pol::msg::QueryMsg::HolderAddress {})
            .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_liquid_pooler_address(rb: &ChainRequestBuilder, contract_address: &str) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(
            &valence_covenant_two_party_pol::msg::QueryMsg::LiquidPoolerAddress {},
        )
        .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_interchain_router_address(
    rb: &ChainRequestBuilder,
    contract_address: &str,
    party: String,
) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(
            &valence_covenant_two_party_pol::msg::QueryMsg::InterchainRouterAddress { party },
        )
        .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_ibc_forwarder_address(
    rb: &ChainRequestBuilder,
    contract_address: &str,
    party: String,
) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(
            &valence_covenant_two_party_pol::msg::QueryMsg::IbcForwarderAddress { party },
        )
        .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_deposit_address(
    rb: &ChainRequestBuilder,
    contract_address: &str,
    party: String,
) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(
            &valence_covenant_two_party_pol::msg::QueryMsg::PartyDepositAddress { party },
        )
        .unwrap(),
    );
    query_response["data"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub fn query_contract_state(rb: &ChainRequestBuilder, contract_address: &str) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(&Messages::ContractState {}).unwrap(),
    );
    query_response["data"].as_str().unwrap().to_string()
}

pub fn tick(rb: &ChainRequestBuilder, from_key: &str, contract_address: &str) {
    contract_execute(
        rb,
        contract_address,
        from_key,
        &serde_json::to_string(&Messages::Tick {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
}
