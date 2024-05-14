use localic_std::{filesystem::get_files, transactions::ChainRequestBuilder};

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
