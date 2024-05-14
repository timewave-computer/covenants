use std::path::PathBuf;
use std::{collections::HashMap, path};

use localic_std::{modules::cosmwasm::CosmWasm, transactions::ChainRequestBuilder};

use crate::types::ChainsVec;
use crate::utils::API_URL;

pub struct TestContext {
    pub chains: HashMap<String, LocalChain>,
}

impl From<ChainsVec> for TestContext {
    fn from(chains: ChainsVec) -> Self {
        let mut chains_map = HashMap::new();
        for chain in chains.chains {
            let rb = ChainRequestBuilder::new(
                API_URL.to_string(),
                chain.chain_id.clone(),
                chain.debugging,
            )
            .unwrap();
            let (src_addr, denom) = match rb.chain_id.as_str() {
                "localneutron-1" => ("neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky", "untrn"),
                "localcosmos-1" => ("cosmos1hj5fveer5cjtn4wd6wstzugjfdxzl0xpxvjjvr", "uatom"),
                _ => ("err", "err"),
            };
            let local_chain = LocalChain::new(rb, src_addr.to_string(), denom.to_string());
            chains_map.insert(chain.name.clone(), local_chain);
        }
        Self { chains: chains_map }
    }
}

pub struct LocalChain {
    /// ChainRequestBuilder
    pub rb: ChainRequestBuilder,
    /// contract codes stored on this chain (filename -> code_id)
    pub contract_codes: HashMap<String, u64>,
    /// outgoing channel ids available (dest_chain_id -> channel_id)
    pub channel_ids: HashMap<String, String>,
    /// outgoing connection ids available (dest_chain_id -> connection_id)
    pub connection_ids: HashMap<String, String>,
    pub admin_addr: String,
    pub native_denom: String,
}

impl LocalChain {
    pub fn new(rb: ChainRequestBuilder, admin_addr: String, native_denom: String) -> Self {
        Self {
            rb,
            contract_codes: Default::default(),
            channel_ids: Default::default(),
            connection_ids: Default::default(),
            admin_addr,
            native_denom,
        }
    }

    pub fn get_cw(&mut self) -> CosmWasm {
        CosmWasm::new(&self.rb)
    }

    pub fn save_code(&mut self, abs_path: PathBuf, code: u64) {
        let id = abs_path.file_stem().unwrap().to_str().unwrap();
        self.contract_codes.insert(id.to_string(), code);
    }
}

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
