use cosmwasm_schema::cw_serde;
use localic_std::{errors::LocalError, transactions::ChainRequestBuilder};
use serde_json::{json, Value};

use crate::pretty_print;

pub fn query_stakeibc_validators(
    chain: &ChainRequestBuilder,
    chain_id: &str,
) -> StakeIbcValsResponse {
    let query_stakeibc_vals_cmd = format!("stakeibc show-validators {chain_id} --output=json",);
    let query_stakeibc_vals_response = chain.q(&query_stakeibc_vals_cmd, false);

    let stake_ibc_vals_response: StakeIbcValsResponse =
        serde_json::from_value(query_stakeibc_vals_response).unwrap();
    stake_ibc_vals_response
}

#[cw_serde]
pub struct StakeIbcValsResponse {
    pub validators: Vec<StakeIbcVal>,
}

#[cw_serde]
pub struct StakeIbcVal {
    pub address: String,
    pub delegation_amt: String,
    pub internal_exchange_rate: Option<String>,
    pub name: String,
    pub weight: String,
}

pub fn query_host_zone(rb: &ChainRequestBuilder, chain_id: &str) -> bool {
    let query_cmd = format!("stakeibc show-host-zone {chain_id} --output=json");
    let host_zone_query_response = rb.q(&query_cmd, false);
    println!("\nhost_zone_query_response:\n");
    pretty_print(&host_zone_query_response);

    host_zone_query_response["host_zone"].is_object()
}

pub fn format_autopilot_string(new_receiver: String) -> String {
    json!({
        "autopilot": {
            "receiver": format!("{new_receiver}"),
            "stakeibc": {
                "action": "LiquidStake"
            }
        },
    })
    .to_string()
}

pub fn add_stakeibc_validator(
    chain: &ChainRequestBuilder,
    config_path: &str,
    validator_chain_id: &str,
) {
    let add_vals_cmd = format!(
        "tx stakeibc add-validators {validator_chain_id} {config_path} --from=admin --gas auto --gas-adjustment 1.3 --output=json",
    );
    let add_vals_response = chain.tx(&add_vals_cmd, false).unwrap();
    println!("\nadd_vals_response:\n");
    pretty_print(&add_vals_response);
}

pub fn register_stride_host_zone(
    rb: &ChainRequestBuilder,
    connection_id: &str,
    host_denom: &str,
    bech_32_prefix: &str,
    ibc_denom: &str,
    channel_id: &str,
    from_key: &str,
) -> Result<Value, LocalError> {
    let cmd = format!(
        "tx stakeibc register-host-zone {} {} {} {} {} 1 --from={} --gas auto --gas-adjustment 1.3 --output=json",
        connection_id,
        host_denom,
        bech_32_prefix,
        ibc_denom,
        channel_id,
        from_key,
    );
    rb.tx(&cmd, true)
}
