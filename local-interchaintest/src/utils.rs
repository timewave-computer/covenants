use std::{
    fs::{self, DirEntry},
    io,
};

use crate::types::ChainsVec;

pub const API_URL: &str = "http://127.0.0.1:42069";
pub const ADMIN_KEY: &str = "acc0";
pub const WASM_EXTENSION: &str = "wasm";
pub const NEUTRON_CHAIN: &str = "neutron";
pub const CHAIN_CONFIG_PATH: &str = "chains/neutron_gaia.json";
pub const ARTIFACTS_PATH: &str = "../artifacts";

pub fn read_json_file(file_path: &str) -> Result<ChainsVec, io::Error> {
    // Read the file to a string
    let data = fs::read_to_string(file_path)?;

    // Parse the string into the struct
    let chain: ChainsVec = serde_json::from_str(&data)?;

    Ok(chain)
}

pub fn read_artifacts(path: &str) -> Result<Vec<DirEntry>, io::Error> {
    let artifacts = fs::read_dir(path).unwrap();

    let mut dir_entries = vec![];
    for dir in artifacts.into_iter() {
        dir_entries.push(dir.unwrap());
    }

    Ok(dir_entries)
}
