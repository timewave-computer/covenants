#![allow(dead_code, unused_must_use)]

use cosmwasm_std::{Coin, Uint128};
use local_ictest_e2e::{
    base::TestContext,
    chain_tests::test_paths,
    utils::{
        read_artifacts, read_json_file, ADMIN_KEY, API_URL, ARTIFACTS_PATH, CHAIN_CONFIG_PATH,
        NEUTRON_CHAIN, WASM_EXTENSION,
    },
};
use localic_std::{
    modules::{
        bank::{get_balance, get_total_supply, send},
        cosmwasm::CosmWasm,
    }, node::Chain, polling::poll_for_start, relayer::Relayer, transactions::ChainRequestBuilder
};
use reqwest::blocking::Client;

// local-ic start neutron_gaia --api-port 42069
fn main() {
    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let mut test_ctx = TestContext::from(configured_chains);

    println!("transfer channels: {:?}", test_ctx.transfer_channel_ids);
    let wasm_files = read_artifacts(ARTIFACTS_PATH).unwrap();

    for wasm_file in wasm_files {
        let path = wasm_file.path();
        // TODO: need to work out caching here eventually
        // TODO: split contracts by chain
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let neutron_local_chain = test_ctx.chains.get_mut(NEUTRON_CHAIN).unwrap();

            let mut cw = CosmWasm::new(&neutron_local_chain.rb);

            let code_id = cw.store(ADMIN_KEY, abs_path.as_path()).unwrap();

            let id = abs_path.file_stem().unwrap().to_str().unwrap();
            neutron_local_chain
                .contract_codes
                .insert(id.to_string(), code_id);
            break; // for testing
        }
    }

    println!(
        "Contract codes: {:?}",
        test_ctx.chains.get(NEUTRON_CHAIN).unwrap().contract_codes
    );

    test_ctx.chains.iter().for_each(|(name, chain)| {
        println!("Chain: {}", name);
        test_paths(&chain.rb);
        test_queries(&chain.rb);
        test_bank_send(&chain.rb, &chain.admin_addr, &chain.native_denom);
    });

    test_ibc_transfer(&test_ctx);
}

fn test_ibc_transfer(test_ctx: &TestContext) {
    let gaia = test_ctx.chains.get("gaia").unwrap();
    let neutron = test_ctx.chains.get("neutron").unwrap();
    let stride = test_ctx.chains.get("stride").unwrap();

    let neutron_relayer = Relayer::new(&neutron.rb);
    let gaia_relayer = Relayer::new(&gaia.rb);
    let stride_relayer = Relayer::new(&stride.rb);

    let neutron_channels = neutron_relayer.get_channels(neutron.rb.chain_id.as_str()).unwrap();
    let gaia_channels = gaia_relayer.get_channels(gaia.rb.chain_id.as_str()).unwrap();
    let stride_channels = stride_relayer.get_channels(stride.rb.chain_id.as_str()).unwrap();

    println!("Neutron channels: {:?}", neutron_channels);
    println!("Gaia channels: {:?}", gaia_channels);
    println!("Stride channels: {:?}", stride_channels);


}

fn test_bank_send(rb: &ChainRequestBuilder, src_addr: &str, denom: &str) {
    let before_bal = get_balance(rb, src_addr);

    let res = send(
        rb,
        "acc0",
        src_addr,
        &[Coin {
            denom: denom.to_string(),
            amount: Uint128::new(5),
        }],
        &Coin {
            denom: denom.to_string(),
            amount: Uint128::new(5000),
        },
    );
    match res {
        Ok(res) => {
            println!("res: {res}");
        }
        Err(err) => {
            println!("err: {err}");
        }
    }

    let after_amount = get_balance(rb, src_addr);

    println!("before: {before_bal:?}");
    println!("after: {after_amount:?}");
}

fn test_queries(rb: &ChainRequestBuilder) {
    test_all_accounts(rb);
    let c = get_total_supply(rb);
    println!("total supply: {c:?}");
}

fn test_all_accounts(rb: &ChainRequestBuilder) {
    let res = rb.query("q auth accounts", false);
    println!("res: {res}");

    let Some(accounts) = res["accounts"].as_array() else {
        println!("No accounts found.");
        return;
    };

    for account in accounts.iter() {
        let acc_type = account["@type"].as_str().unwrap_or_default();

        let addr: &str = match acc_type {
            // "/cosmos.auth.v1beta1.ModuleAccount" => account["base_account"]["address"]
            "/cosmos.auth.v1beta1.ModuleAccount" => account.get("base_account").unwrap()["address"]
                .as_str()
                .unwrap_or_default(),
            _ => account["address"].as_str().unwrap_or_default(),
        };

        println!("{acc_type}: {addr}");
    }
}

fn get_keyring_accounts(rb: &ChainRequestBuilder) {
    let accounts = rb.binary("keys list --keyring-backend=test", false);

    let addrs = accounts["addresses"].as_array();
    addrs.map_or_else(
        || {
            println!("No accounts found.");
        },
        |addrs| {
            for acc in addrs.iter() {
                let name = acc["name"].as_str().unwrap_or_default();
                let address = acc["address"].as_str().unwrap_or_default();
                println!("Key '{name}': {address}");
            }
        },
    );
}
