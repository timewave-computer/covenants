use localic_std::modules::cosmwasm::CosmWasm;

use super::{
    constants::{ACC_0_KEY, WASM_EXTENSION},
    file_system::read_artifacts,
    test_context::TestContext,
};

pub fn deploy_contracts_on_chain(test_ctx: &mut TestContext, path: &str, chain: &str) {
    let dir_entries = read_artifacts(path).unwrap();

    for entry in dir_entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some(WASM_EXTENSION) {
            let abs_path = path.canonicalize().unwrap();
            let local_chain = test_ctx.get_mut_chain(chain);

            let mut cw = CosmWasm::new(&local_chain.rb);
            let id = abs_path.file_stem().unwrap().to_str().unwrap();

            //To avoid storing multiple times during the same execution
            if local_chain.contract_codes.contains_key(id) {
                println!(
                    "Contract {} already deployed on chain {}, skipping...",
                    id, chain
                );
                continue;
            }

            let code_id = cw.store(ACC_0_KEY, abs_path.as_path()).unwrap();

            local_chain.contract_codes.insert(id.to_string(), code_id);
        }
    }
}
