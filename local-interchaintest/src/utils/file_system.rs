use std::{
    fs::{self, DirEntry},
    io::{self, Write},
    path::{self, Path},
};

use localic_std::transactions::ChainRequestBuilder;

use super::types::ChainsVec;

/// Will panic if the current directory path is not found.
#[must_use]
pub fn get_current_dir() -> path::PathBuf {
    match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => panic!("Could not get current dir: {e}"),
    }
}

/// Will panic if the `local_interchain` directory is not found in the parent path.
#[must_use]
pub fn get_local_interchain_dir() -> path::PathBuf {
    let current_dir = get_current_dir();
    let Some(parent_dir) = current_dir.parent() else { panic!("Could not get parent dir") };
    parent_dir.to_path_buf()
}

/// local-interchain/contracts directory
#[must_use]
pub fn get_contract_path() -> path::PathBuf {
    get_local_interchain_dir().join("contracts")
}

/// local-interchain/configs/contract.json file
#[must_use]
pub fn get_contract_cache_path() -> path::PathBuf {
    get_local_interchain_dir()
        .join("configs")
        .join("contract.json")
}

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

pub fn write_json_file(path: &str, data: &str) {
    let path = Path::new(path);
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(data.as_bytes()).unwrap();

    println!("file written: {:?}", path);
}

pub fn write_str_to_container_file(rb: &ChainRequestBuilder, container_path: &str, content: &str) {
    // TODO: fix this. perhaps draw inspiration from request_builder upload_file.
    let filewriting = rb.exec(
        &format!("/bin/sh -c echo '{}' > {}", content, container_path),
        true,
    );
    println!("filewriting: {:?}", filewriting);
}
