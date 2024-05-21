use std::path::PathBuf;
use std::{collections::HashMap, path};

use localic_std::relayer::{Channel, Relayer};
use localic_std::{modules::cosmwasm::CosmWasm, transactions::ChainRequestBuilder};

use crate::chain_tests::{find_pairwise_ccv_channel_ids, find_pairwise_transfer_channel_ids};
use crate::ibc_helpers;
use crate::types::ChainsVec;
use crate::utils::API_URL;

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
