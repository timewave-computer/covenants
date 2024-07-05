use localic_std::{
    modules::cosmwasm::{contract_execute, contract_query},
    transactions::ChainRequestBuilder,
};
use serde::{Deserialize, Serialize};

use crate::utils::constants::EXECUTE_FLAGS;

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
