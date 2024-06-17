use localic_std::modules::cosmwasm::CosmWasm;

use crate::{
    utils::{file_system::read_artifacts, test_context::TestContext},
    ACC_0_KEY, ARTIFACTS_PATH, NEUTRON_CHAIN, WASM_EXTENSION,
};

pub fn deploy_neutron_contracts(test_ctx: &mut TestContext) {
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
}
