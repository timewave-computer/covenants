#![allow(dead_code, unused_must_use)]

use local_ictest_e2e::{
    base::TestContext,
    utils::{
        read_artifacts, read_json_file, ADMIN_KEY, API_URL, ARTIFACTS_PATH, CHAIN_CONFIG_PATH,
        NEUTRON_CHAIN, WASM_EXTENSION,
    },
};
use localic_std::{modules::cosmwasm::CosmWasm, polling::poll_for_start};
use reqwest::blocking::Client;

// local-ic start neutron_gaia --api-port 42069
fn main() {
    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let mut test_ctx = TestContext::from(configured_chains);

    let neutron = test_ctx.chains.get_mut(NEUTRON_CHAIN).unwrap();

    let wasm_files = read_artifacts(ARTIFACTS_PATH).unwrap();
    for wasm_file in wasm_files {
        let path = wasm_file.path();
        // TODO: need to work out caching here eventually
        // TODO: split contracts by chain
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let mut cw = CosmWasm::new(&neutron.rb);

            let code_id = cw.store(ADMIN_KEY, abs_path.as_path()).unwrap();

            let id = abs_path.file_stem().unwrap().to_str().unwrap();
            neutron.contract_codes.insert(id.to_string(), code_id);
        }
    }

    println!(
        "Contract codes: {:?}",
        test_ctx.chains.get(NEUTRON_CHAIN).unwrap().contract_codes
    );
}
