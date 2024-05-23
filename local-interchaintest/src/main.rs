#![allow(dead_code, unused_must_use)]

use cosmwasm_std::{coin, Coin, Uint128};
use local_ictest_e2e::{
    utils::{
        file_system::{
            get_contract_cache_path, get_contract_path, get_current_dir, get_local_interchain_dir,
            read_artifacts, read_json_file, write_json_file, write_str_to_container_file,
        },
        ibc::{get_ibc_denom, ibc_send},
        queries::{query_validator_set, ValidatorsJson},
        stride::{
            add_stakeibc_validator, format_autopilot_string, query_host_zone,
            query_stakeibc_validators, register_stride_host_zone,
        },
        test_context::TestContext,
    },
    ACC_0_KEY, ADMIN_KEY, API_URL, ARTIFACTS_PATH, CHAIN_CONFIG_PATH, GAIA_CHAIN, GAIA_CHAIN_ID,
    NEUTRON_CHAIN, STRIDE_CHAIN, STRIDE_CHAIN_ID, WASM_EXTENSION,
};
use localic_std::{
    filesystem::get_files,
    modules::{
        bank::{get_balance, get_total_supply, send},
        cosmwasm::CosmWasm,
    },
    polling::poll_for_start,
    relayer::Relayer,
    transactions::ChainRequestBuilder,
};
use reqwest::blocking::Client;

// local-ic start neutron_gaia --api-port 42069
fn main() {
    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let mut test_ctx = TestContext::from(configured_chains);

    let wasm_files = read_artifacts(ARTIFACTS_PATH).unwrap();

    for wasm_file in wasm_files {
        let path = wasm_file.path();
        // TODO: need to work out caching here eventually
        // TODO: split contracts by chain
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let neutron_local_chain = test_ctx.get_mut_chain(NEUTRON_CHAIN);

            let mut cw = CosmWasm::new(&neutron_local_chain.rb);

            let code_id = cw.store(ACC_0_KEY, abs_path.as_path()).unwrap();

            let id = abs_path.file_stem().unwrap().to_str().unwrap();
            neutron_local_chain
                .contract_codes
                .insert(id.to_string(), code_id);
            break; // for testing
        }
    }

    let stride = test_ctx.get_chain(STRIDE_CHAIN);

    let stride_to_gaia_channel_id = test_ctx
        .get_transfer_channels()
        .src(STRIDE_CHAIN)
        .dest(GAIA_CHAIN)
        .get();
    let atom_on_stride = get_ibc_denom("uatom", &stride_to_gaia_channel_id);

    ibc_send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN),
        ACC_0_KEY,
        &test_ctx.get_admin_addr().src(STRIDE_CHAIN).get(),
        coin(100, "uatom"),
        &coin(100, "uatom"),
        &test_ctx
            .get_transfer_channels()
            .src(GAIA_CHAIN)
            .dest(STRIDE_CHAIN)
            .get(),
        None,
    )
    .unwrap();

    if query_host_zone(&stride.rb, GAIA_CHAIN_ID) {
        println!("Host zone registered.");
    } else {
        println!("Host zone not registered.");
        register_stride_host_zone(
            test_ctx
                .get_request_builder()
                .get_request_builder(STRIDE_CHAIN),
            &test_ctx
                .get_connections()
                .src(STRIDE_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            "uatom",
            "cosmos",
            &atom_on_stride,
            &stride_to_gaia_channel_id,
            ADMIN_KEY,
        )
        .unwrap();
    }

    register_gaia_validators_on_stride(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN),
        test_ctx
            .get_request_builder()
            .get_request_builder(STRIDE_CHAIN),
    );

    ibc_send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN),
        ACC_0_KEY,
        &test_ctx.get_admin_addr().src(STRIDE_CHAIN).get(),
        coin(10000, "uatom"),
        &coin(10000, "uatom"),
        &test_ctx
            .get_transfer_channels()
            .src(GAIA_CHAIN)
            .dest(STRIDE_CHAIN)
            .get(),
        Some(&format_autopilot_string(stride.admin_addr.to_string())),
    )
    .unwrap();

    let stride_bal: Vec<Coin> = get_balance(&stride.rb, &stride.admin_addr)
        .into_iter()
        .filter(|c| c.denom == atom_on_stride)
        .collect();

    println!("post autopilot stride acc balance: {:?}", stride_bal);
}

pub fn register_gaia_validators_on_stride(
    gaia: &ChainRequestBuilder,
    stride: &ChainRequestBuilder,
) {
    let val_set_entries = query_validator_set(gaia);

    if query_stakeibc_validators(stride, GAIA_CHAIN_ID)
        .validators
        .is_empty()
    {
        println!("Validators registered.");
        return;
    }

    let validators_json = serde_json::to_value(ValidatorsJson {
        validators: val_set_entries,
    })
    .unwrap();

    println!("validators_json: {:?}", validators_json.to_string());
    write_json_file("validators.json", &validators_json.to_string());

    let stride_path = format!("/var/cosmos-chain/{STRIDE_CHAIN_ID}/config/validators.json");

    write_str_to_container_file(stride, "validators.json", &validators_json.to_string());

    add_stakeibc_validator(stride, &stride_path, GAIA_CHAIN_ID);

    let stakeibc_vals_response = query_stakeibc_validators(stride, GAIA_CHAIN_ID);
    if stakeibc_vals_response.validators.is_empty() {
        println!("Validators not registered.");
    } else {
        println!("Validators registered.");
    }
}

fn test_ibc_transfer(test_ctx: &TestContext) {
    let gaia = test_ctx.get_chain(GAIA_CHAIN);
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN);
    let stride = test_ctx.get_chain(STRIDE_CHAIN);

    let neutron_relayer = Relayer::new(&neutron.rb);
    let gaia_relayer = Relayer::new(&gaia.rb);
    let stride_relayer = Relayer::new(&stride.rb);

    let neutron_channels = neutron_relayer
        .get_channels(neutron.rb.chain_id.as_str())
        .unwrap();
    let gaia_channels = gaia_relayer
        .get_channels(gaia.rb.chain_id.as_str())
        .unwrap();
    let stride_channels = stride_relayer
        .get_channels(stride.rb.chain_id.as_str())
        .unwrap();

    println!("Neutron channels: {:?}", neutron_channels);
    println!("Gaia channels: {:?}", gaia_channels);
    println!("Stride channels: {:?}", stride_channels);
}

fn test_bank_send(rb: &ChainRequestBuilder, src_addr: &str, denom: &str) {
    let before_bal = get_balance(rb, src_addr);

    let res = send(
        rb,
        ACC_0_KEY,
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
