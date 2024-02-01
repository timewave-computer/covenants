use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    instantiate2_address, Addr, Binary, CanonicalAddr, CodeInfoResponse, Deps, StdError, StdResult,
};
use sha2::{Digest, Sha256};

fn get_precomputed_address(
    deps: Deps,
    code_id: u64,
    creator: &CanonicalAddr,
    salt: &[u8],
) -> StdResult<Addr> {
    let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;

    match instantiate2_address(&checksum, creator, salt) {
        Ok(addr) => Ok(deps.api.addr_humanize(&addr)?),
        Err(e) => Err(StdError::generic_err(e.to_string())),
    }
}

fn generate_contract_salt(salt_str: &[u8]) -> Binary {
    let mut hasher = Sha256::new();
    hasher.update(salt_str);
    hasher.finalize().to_vec().into()
}

pub fn get_instantiate2_salt_and_address(
    deps: Deps,
    salt_bytes: &[u8],
    creator_address: &CanonicalAddr,
    code_id: u64,
) -> StdResult<Instantiate2HelperConfig> {
    let salt_binary = generate_contract_salt(salt_bytes);

    let contract_instantiate2_address =
        get_precomputed_address(deps, code_id, creator_address, &salt_binary)?;

    Ok(Instantiate2HelperConfig {
        addr: contract_instantiate2_address,
        code: code_id,
        salt: salt_binary,
    })
}

#[cw_serde]
pub struct Instantiate2HelperConfig {
    pub addr: Addr,
    pub code: u64,
    pub salt: Binary,
}
