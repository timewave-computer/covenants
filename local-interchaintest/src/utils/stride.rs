use localic_std::{errors::LocalError, transactions::ChainRequestBuilder};
use serde_json::Value;

use crate::utils::file_system::pretty_print;

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

pub fn query_host_zone(rb: &ChainRequestBuilder, chain_id: &str) -> bool {
    let query_cmd = format!("stakeibc show-host-zone {chain_id} --output=json");
    let host_zone_query_response = rb.q(&query_cmd, false);
    println!("\nhost_zone_query_response:\n");
    pretty_print(&host_zone_query_response);

    host_zone_query_response["host_zone"].is_object()
}
