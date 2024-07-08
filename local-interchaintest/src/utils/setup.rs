use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
};

use localic_std::modules::cosmwasm::CosmWasm;
use log::{error, info};

use super::{
    constants::{ACC_0_KEY, LOCAL_CODE_ID_CACHE_PATH, WASM_EXTENSION},
    file_system::read_artifacts,
    test_context::TestContext,
};

pub fn deploy_contracts_on_chain(test_ctx: &mut TestContext, path: &str, chain: &str) {
    if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
        info!("Path {} exists, deploying contracts...", path);
    } else {
        error!(
            "Path {} does not exist, you might have to build and optimize contracts",
            path
        );
        return;
    };
    let dir_entries = read_artifacts(path).unwrap();

    // Use a local cache to avoid storing the same contract multiple times, useful for local testing
    let mut content = String::new();
    let cache: HashMap<String, u64> = match File::open(LOCAL_CODE_ID_CACHE_PATH) {
        Ok(mut file) => {
            if let Err(err) = file.read_to_string(&mut content) {
                error!("Failed to read cache file: {}", err);
                HashMap::new()
            } else {
                serde_json::from_str(&content).unwrap_or_default()
            }
        }
        Err(_) => {
            // If the file does not exist, we'll create it later
            HashMap::new()
        }
    };

    let local_chain = test_ctx.get_mut_chain(chain);
    // Add all cache entries to the local chain
    for (id, code_id) in cache {
        local_chain.contract_codes.insert(id, code_id);
    }

    for entry in dir_entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let mut cw = CosmWasm::new(&local_chain.rb);
            let id = abs_path.file_stem().unwrap().to_str().unwrap();

            // To avoid storing multiple times during the same execution
            if local_chain.contract_codes.contains_key(id) {
                info!(
                    "Contract {} already deployed on chain {}, skipping...",
                    id, chain
                );
                continue;
            }

            let code_id = cw.store(ACC_0_KEY, abs_path.as_path()).unwrap();

            local_chain.contract_codes.insert(id.to_string(), code_id);
        }
    }

    let contract_codes = serde_json::to_string(&local_chain.contract_codes).unwrap();
    let mut file = File::create(LOCAL_CODE_ID_CACHE_PATH).unwrap();
    file.write_all(contract_codes.as_bytes()).unwrap();
}
