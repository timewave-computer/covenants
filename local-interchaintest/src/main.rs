#![allow(dead_code, unused_must_use)]

use std::{io::Write, path::Path};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Coin, Uint128};
use local_ictest_e2e::{
    ibc_helpers::{get_ibc_denom, ibc_send}, test_context::TestContext, utils::{
        read_artifacts, read_json_file, ADMIN_KEY, API_URL, ARTIFACTS_PATH, CHAIN_CONFIG_PATH,
        NEUTRON_CHAIN, WASM_EXTENSION,
    }
};
use localic_std::{
    errors::LocalError, modules::{
        bank::{get_balance, get_total_supply, send},
        cosmwasm::CosmWasm,
    }, polling::poll_for_start, relayer::Relayer, transactions::ChainRequestBuilder,
};
use reqwest::blocking::Client;
use serde_json::{json, Value};


// local-ic start neutron_gaia --api-port 42069
fn main() {
    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let client = Client::new();
    poll_for_start(&client, API_URL, 300);


    let mut test_ctx = TestContext::from(configured_chains);

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

    let stride = test_ctx.chains.get("stride").unwrap();

    let src_port = "transfer";

    let stride_to_gaia_channel_id = test_ctx.get_transfer_channels().src("stride").dest("gaia").get();
    let atom_on_stride = get_ibc_denom("uatom", &stride_to_gaia_channel_id);

    ibc_send(
        &test_ctx.get_request_builder().get_request_builder("gaia"),
        "acc0",
        &test_ctx.get_admin_addr().src("stride").get(),
        coin(100, "uatom"),
        &coin(100, "uatom"),
        src_port,
        &test_ctx.get_transfer_channels().src("gaia").dest("stride").get(),
        None,
    ).unwrap();


    if query_host_zone(&stride.rb, "localcosmos-1") {
        println!("Host zone registered.");
    } else {
        println!("Host zone not registered.");
        register_stride_host_zone(
            &test_ctx.get_request_builder().get_request_builder("stride"),
            &test_ctx.get_connections().src("stride").dest("gaia").get(),
            "uatom",
            "cosmos",
            &atom_on_stride,
            &stride_to_gaia_channel_id,
            1,
            "admin",
        )
        .unwrap();
    }

    register_gaia_validators_on_stride(
        &test_ctx.get_request_builder().get_request_builder("gaia"),
        &test_ctx.get_request_builder().get_request_builder("stride"),
    );

    let autopilot_str = format_autopilot_string(stride.admin_addr.to_string());

    ibc_send(
        &test_ctx.get_request_builder().get_request_builder("gaia"),
        "acc0",
        &test_ctx.get_admin_addr().src("stride").get(),
        coin(10000, "uatom"),
        &coin(10000, "uatom"),
        src_port,
        &test_ctx.get_transfer_channels().src("gaia").dest("stride").get(),
        Some(&autopilot_str),
    ).unwrap();

    let stride_bal: Vec<Coin> = get_balance(&stride.rb, &stride.admin_addr)
        .into_iter()
        .filter(|c| c.denom == atom_on_stride)
        .collect();

    println!("post autopilot stride acc balance: {:?}", stride_bal);
}

fn format_autopilot_string(new_receiver: String) -> String {
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

#[cw_serde]
pub struct ValidatorSetEntry {
    pub address: String,
    pub voting_power: String,
    pub name: String,
}

#[cw_serde]
pub struct ValidatorsJson {
    pub validators: Vec<ValidatorSetEntry>,
}


fn write_json_file(path: &str, data: &str) {
    let path = Path::new(path);
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(data.as_bytes()).unwrap();

    println!("file written: {:?}", path);
}

pub fn query_validator_set(chain: &ChainRequestBuilder) -> Vec<ValidatorSetEntry> {
    let height = query_block_height(chain);
    let query_valset_cmd = format!(
        "tendermint-validator-set {height} --output=json",
    );

    let valset_resp = chain.q(&query_valset_cmd, false);

    let mut val_set_entries: Vec<ValidatorSetEntry> = Vec::new();

    for entry in valset_resp["validators"].as_array().unwrap() {
        let address = entry["address"].as_str().unwrap();
        let voting_power = entry["voting_power"].as_str().unwrap();

        val_set_entries.push(ValidatorSetEntry {
            name: format!("val{}", val_set_entries.len() + 1),
            address: address.to_string(),
            voting_power: voting_power.to_string(),
        });
    }
    val_set_entries
}

pub fn write_str_to_container_file(
    rb: &ChainRequestBuilder,
    container_path: &str,
    content: &str,
) {
    // TODO: fix this. perhaps draw inspiration from request_builder upload_file.
    let filewriting = rb.exec(
        &format!("/bin/sh -c echo '{}' > {}", content, container_path),
        true
    );
    println!("filewriting: {:?}", filewriting);
}

pub fn register_gaia_validators_on_stride(
    gaia: &ChainRequestBuilder,
    stride: &ChainRequestBuilder,
) {
    let val_set_entries = query_validator_set(gaia);


    if query_stakeibc_validators(stride).validators.len() != 0 {
        println!("Validators registered.");
        return;
    }


    let validators_json = serde_json::to_value(ValidatorsJson {
        validators: val_set_entries,
    })
    .unwrap();

    println!("validators_json: {:?}", validators_json.to_string());
    write_json_file("validators.json", &validators_json.to_string());

    let stride_path = "/var/cosmos-chain/localstride-3/config/validators.json";

    write_str_to_container_file(stride, &"validators.json", &validators_json.to_string());

    add_stakeibc_validator(stride, &stride_path);

    let stakeibc_vals_response = query_stakeibc_validators(stride);
    if stakeibc_vals_response.validators.len() == 0 {
        println!("Validators not registered.");
    } else {
        println!("Validators registered.");
    }
}

fn add_stakeibc_validator(
    chain: &ChainRequestBuilder,
    config_path: &str,
) {
    let add_vals_cmd = format!(
        "tx stakeibc add-validators localcosmos-1 {config_path} --from=admin --gas auto --gas-adjustment 1.3 --output=json",
    );
    let add_vals_response = chain.tx(&add_vals_cmd, false).unwrap();

    println!("add_val_response: {:?}", add_vals_response);
}

fn query_stakeibc_validators(chain: &ChainRequestBuilder) -> StakeIbcValsResponse {
    let query_stakeibc_vals_cmd = format!(
        "stakeibc show-validators localcosmos-1 --output=json",
    );
    let query_stakeibc_vals_response = chain.q(&query_stakeibc_vals_cmd, false);

    let stake_ibc_vals_response: StakeIbcValsResponse = serde_json::from_value(query_stakeibc_vals_response).unwrap();
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

pub fn query_block_height(chain: &ChainRequestBuilder) -> u64 {
    let query_cmd = format!("block --output=json");
    let mut query_block_response = chain.q(&query_cmd, false);
    // let block_height = &chain_status_response.take()[0]["block"];
    // println!("block response : {:?}", block_height);

    // let block_height = chain_status_response["block"]["header"]["height"].as_u64().unwrap();

    // println!("chain status query response: {:?}", block_height);
    // block_height
    100
}

pub fn query_host_zone(
    rb: &ChainRequestBuilder,
    chain_id: &str,
) -> bool {
    let query_cmd = format!("stakeibc show-host-zone {chain_id} --output=json");
    let host_zone_query_response = rb.q(&query_cmd, false);
    println!("host_zone_query_response: {:?}", host_zone_query_response);

    if host_zone_query_response["host_zone"].is_object() {
        return true;
    } else {
        false
    }
}

pub fn register_stride_host_zone(
    rb: &ChainRequestBuilder,
    connection_id: &str,
    host_denom: &str,
    bech_32_prefix: &str,
    ibc_denom: &str,
    channel_id: &str,
    unbonding_frequency: u64,
    from_key: &str,
) -> Result<Value, LocalError> {
    let cmd = format!(
        "tx stakeibc register-host-zone {} {} {} {} {} {} --from={} --gas auto --gas-adjustment 1.3 --output=json",
        connection_id,
        host_denom,
        bech_32_prefix,
        ibc_denom,
        channel_id,
        unbonding_frequency,
        from_key,
    );
    let res = rb.tx(&cmd, true);
    res
}

fn test_ibc_transfer(test_ctx: &TestContext) {
    let gaia = test_ctx.chains.get("gaia").unwrap();
    let neutron = test_ctx.chains.get("neutron").unwrap();
    let stride = test_ctx.chains.get("stride").unwrap();

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
