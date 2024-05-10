#![allow(dead_code, unused_must_use)]

use std::collections::HashMap;

use local_ictest_e2e::utils::{read_artifacts, read_json_file};
use localic_std::{
    modules::cosmwasm::CosmWasm, polling::poll_for_start, transactions::ChainRequestBuilder,
};
use reqwest::blocking::Client;

const API_URL: &str = "http://127.0.0.1:42069";
const ADMIN_KEY: &str = "acc0";
const WASM_EXTENSION: &str = "wasm";
const NEUTRON_CHAIN: &str = "neutron";

// local-ic start neutron_gaia --api-port 42069
fn main() {
    let configured_chains = match read_json_file("chains/neutron_gaia.json") {
        Ok(chains) => chains,
        Err(e) => panic!("rip: {e}"),
    };

    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let mut chain_map: HashMap<String, ChainRequestBuilder> = HashMap::new();

    for chain in configured_chains.chains {
        match ChainRequestBuilder::new(API_URL.to_string(), chain.chain_id.clone(), chain.debugging)
        {
            Ok(rb) => chain_map.insert(chain.name, rb),
            Err(err) => {
                panic!("ChainRequestBuilder failed: {err:?}");
            }
        };
    }

    let neutron = chain_map.get("neutron").unwrap();

    let mut neutron_cw = CosmWasm::new(neutron);

    let wasm_files = read_artifacts("../artifacts").unwrap();

    let mut code_ids: HashMap<String, u64> = HashMap::new();
    for wasm_file in wasm_files {
        let path = wasm_file.path();
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let file_name = abs_path.file_stem().unwrap().to_str().unwrap();

            let code_id = neutron_cw.store(ADMIN_KEY, abs_path.as_path()).unwrap();
            code_ids.insert(file_name.to_string(), code_id);
        }
    }

    println!("stored code ids: {:?}", code_ids);
}
