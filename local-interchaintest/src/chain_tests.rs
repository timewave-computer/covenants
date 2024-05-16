use cosmwasm_std::{StdError, StdResult};
use localic_std::{filesystem::get_files, relayer::Channel, transactions::ChainRequestBuilder};

use crate::base::{
    get_contract_cache_path, get_contract_path, get_current_dir, get_local_interchain_dir,
};

pub fn test_paths(rb: &ChainRequestBuilder) {
    println!("current_dir: {:?}", get_current_dir());
    println!("local_interchain_dir: {:?}", get_local_interchain_dir());
    println!("contract_path: {:?}", get_contract_path());
    println!("contract_json_path: {:?}", get_contract_cache_path());

    // upload Makefile to the chain's home dir
    let arb_file = get_current_dir().join("Makefile");
    match rb.upload_file(&arb_file, true) {
        Ok(req_builder) => {
            let res = match req_builder.send() {
                Ok(r) => r,
                Err(err) => {
                    panic!("upload_file failed on request send {err:?}");
                }
            };
            let body = match res.text() {
                Ok(body) => body,
                Err(err) => {
                    panic!("upload_file failed on response body {err:?}");
                }
            };
            println!("body: {body:?}");
            let chain_id = rb.chain_id.to_string();
            let assertion_str = format!(
                "{{\"success\":\"file uploaded to {}\",\"location\":\"/var/cosmos-chain/{}/Makefile\"}}",
                chain_id, chain_id
            );
            assert_eq!(body, assertion_str);
        }
        Err(err) => {
            panic!("upload_file failed {err:?}");
        }
    };

    let files = match get_files(rb, format!("/var/cosmos-chain/{}", rb.chain_id).as_str()) {
        Ok(files) => files,
        Err(err) => {
            panic!("get_files failed {err:?}");
        }
    };

    assert!(files.contains(&"Makefile".to_string()));
    assert!(files.contains(&"config".to_string()));
    assert!(files.contains(&"data".to_string()));
    assert!(files.contains(&"keyring-test".to_string()));
    println!("files: {files:?}");
}

pub fn find_pairwise_transfer_channel_ids(
    a: &[Channel],
    b: &[Channel],
) -> StdResult<(PairwiseChannelResult, PairwiseChannelResult)> {
    for (a_i, a_chan) in a.iter().enumerate() {
        for (b_i, b_chan) in b.iter().enumerate() {
            if a_chan.channel_id == b_chan.counterparty.channel_id
                && b_chan.channel_id == a_chan.counterparty.channel_id
                && a_chan.port_id == "transfer"
                && b_chan.port_id == "transfer"
                && a_chan.ordering == "ORDER_UNORDERED"
                && b_chan.ordering == "ORDER_UNORDERED"
            {
                let a_channel_result = PairwiseChannelResult {
                    index: a_i,
                    channel_id: a_chan.channel_id.to_string(),
                    connection_id: a_chan.connection_hops[0].to_string(),
                };
                let b_channel_result = PairwiseChannelResult {
                    index: b_i,
                    channel_id: b_chan.channel_id.to_string(),
                    connection_id: b_chan.connection_hops[0].to_string(),
                };

                return Ok((a_channel_result, b_channel_result));
            }
        }
    }
    Err(StdError::generic_err(
        "failed to match pairwise transfer channels",
    ))
}

pub fn find_pairwise_ccv_channel_ids(
    provider_channels: &[Channel],
    consumer_channels: &[Channel],
) -> StdResult<(PairwiseChannelResult, PairwiseChannelResult)> {
    for (a_i, a_chan) in provider_channels.iter().enumerate() {
        for (b_i, b_chan) in consumer_channels.iter().enumerate() {
            if a_chan.channel_id == b_chan.counterparty.channel_id
                && b_chan.channel_id == a_chan.counterparty.channel_id
                && a_chan.port_id == "provider"
                && b_chan.port_id == "consumer"
                && a_chan.ordering == "ORDER_ORDERED"
                && b_chan.ordering == "ORDER_ORDERED"
            {
                let provider_channel_result = PairwiseChannelResult {
                    index: a_i,
                    channel_id: a_chan.channel_id.to_string(),
                    connection_id: a_chan.connection_hops[0].to_string(),
                };
                let consumer_channel_result = PairwiseChannelResult {
                    index: b_i,
                    channel_id: b_chan.channel_id.to_string(),
                    connection_id: b_chan.connection_hops[0].to_string(),
                };
                return Ok((provider_channel_result, consumer_channel_result));
            }
        }
    }
    Err(StdError::generic_err(
        "failed to match pairwise ccv channels",
    ))
}

pub struct PairwiseChannelResult {
    pub index: usize,
    pub channel_id: String,
    pub connection_id: String,
}
