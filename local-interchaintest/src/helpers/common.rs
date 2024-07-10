use localic_std::{
    modules::cosmwasm::{contract_execute, contract_query},
    transactions::ChainRequestBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::helpers::constants::EXECUTE_FLAGS;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Messages {
    ContractState {},
    Tick {},
}

pub fn query_contract_state(rb: &ChainRequestBuilder, contract_address: &str) -> String {
    let query_response = contract_query(
        rb,
        contract_address,
        &serde_json::to_string(&Messages::ContractState {}).unwrap(),
    );

    if query_response["data"].as_str().is_none() {
        return json!(query_response).to_string();
    }
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
