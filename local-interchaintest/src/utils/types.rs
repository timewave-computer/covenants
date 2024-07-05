use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChainsVec {
    pub chains: Vec<ConfigChain>,
}

#[derive(Deserialize)]
pub struct ConfigChain {
    pub chain_type: Option<String>,
    pub coin_type: i32,
    pub binary: String,
    pub bech32_prefix: String,
    pub denom: String,
    pub trusting_period: String,
    pub debugging: bool,
    pub block_time: String,
    pub host_port_override: Option<HashMap<String, String>>,
    pub ics_consumer_link: Option<String>,

    pub name: String,
    pub chain_id: String,
    pub docker_image: DockerImage,
    pub gas_prices: String,
    pub gas_adjustment: f64,
    pub number_vals: i32,
    pub number_node: i32,
    pub ibc_paths: Option<Vec<String>>,
    pub genesis: Genesis,
    pub config_file_overrides: Option<Vec<ConfigFileOverrides>>,

    // EVM
    pub evm_load_state_path: Option<String>,
}

#[derive(Deserialize)]
pub struct DockerImage {
    pub version: String,
    pub repository: Option<String>,
}

#[derive(Deserialize)]
pub struct Genesis {
    pub modify: Vec<KVStore>,
    pub accounts: Vec<GenesisAccount>,
}

#[derive(Deserialize)]
pub struct KVStore {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct GenesisAccount {
    pub name: String,
    pub amount: String,
    pub address: String,
    pub mnemonic: String,
}

#[derive(Deserialize)]
pub struct ConfigFileOverrides {
    pub file: String,
    pub paths: String,
}

#[derive(Deserialize, Debug)]
pub struct Channel {
    pub channel_id: String,
    pub connection_hops: Vec<String>,
    pub counterparty: Counterparty,
    pub ordering: String,
    pub port_id: String,
    pub state: String,
    pub version: String,
}

#[derive(Deserialize, Debug)]
pub struct Counterparty {
    pub channel_id: String,
    pub port_id: String,
}
