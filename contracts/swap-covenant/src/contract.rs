
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, WasmMsg, CanonicalAddr, instantiate2_address, CodeInfoResponse,
};

use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;
use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_interchain_splitter::msg::PresetInterchainSplitterFields;
use covenant_swap_holder::msg::PresetSwapHolderFields;
use covenant_utils::{CovenantPartiesConfig, CovenantTerms};
use cw2::set_contract_version;
use sha2::{Sha256, Digest};

use crate::{
    error::ContractError,
    msg::{CovenantPartyConfig, InstantiateMsg, QueryMsg},
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_INTERCHAIN_SPLITTER_ADDR, COVENANT_SWAP_HOLDER_ADDR,
        PARTY_A_IBC_FORWARDER_ADDR, PARTY_A_INTERCHAIN_ROUTER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
        PARTY_B_INTERCHAIN_ROUTER_ADDR,
    },
};

const CONTRACT_NAME: &str = "crates.io:swap-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_SALT: &[u8]                     = b"clock";
pub const PARTY_A_INTERCHAIN_ROUTER_SALT: &[u8] = b"party_a_interchain_router";
pub const PARTY_B_INTERCHAIN_ROUTER_SALT: &[u8] = b"party_b_interchain_router";
pub const SPLITTER_SALT: &[u8]                  = b"splitter";
pub const SWAP_HOLDER_SALT: &[u8]               = b"swap_holder";
pub const PARTY_A_FORWARDER_SALT: &[u8]         = b"party_a_ibc_forwarder";
pub const PARTY_B_FORWARDER_SALT: &[u8]         = b"party_b_ibc_forwarder";


pub fn generate_contract_salt(salt_str: &[u8]) -> cosmwasm_std::Binary {
    let mut hasher = Sha256::new();
    hasher.update(salt_str);
    hasher.finalize().to_vec().into()
}

fn get_precomputed_address(
    deps: Deps,
    code_id: u64,
    creator: &CanonicalAddr,
    salt: &[u8],
) -> Result<Addr, ContractError> {
    let CodeInfoResponse {
        checksum,
        ..
    } = deps.querier.query_wasm_code_info(code_id)?;

    let precomputed_address = instantiate2_address(&checksum, &creator, salt)?;

    Ok(deps.api.addr_humanize(&precomputed_address)?)
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let creator_address = deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let clock_salt = generate_contract_salt(CLOCK_SALT);
    let party_a_router_salt = generate_contract_salt(PARTY_A_INTERCHAIN_ROUTER_SALT);
    let party_b_router_salt = generate_contract_salt(PARTY_B_INTERCHAIN_ROUTER_SALT);
    let holder_salt = generate_contract_salt(SWAP_HOLDER_SALT);
    let party_a_forwarder_salt = generate_contract_salt(PARTY_A_FORWARDER_SALT);
    let party_b_forwarder_salt = generate_contract_salt(PARTY_B_FORWARDER_SALT);
    let splitter_salt = generate_contract_salt(SPLITTER_SALT);


    let clock_address = get_precomputed_address(deps.as_ref(), msg.contract_codes.clock_code, &creator_address, &clock_salt)?;
    let party_a_interchain_router_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.interchain_router_code, &creator_address, &party_a_router_salt)?;
    let party_b_interchain_router_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.interchain_router_code, &creator_address, &party_b_router_salt)?;
    let splitter_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.splitter_code, &creator_address, &splitter_salt)?;
    let swap_holder_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.holder_code, &creator_address, &holder_salt)?;
    let party_a_forwarder_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.ibc_forwarder_code, &creator_address, &party_a_forwarder_salt)?;
    let party_b_forwarder_address =
        get_precomputed_address(deps.as_ref(), msg.contract_codes.ibc_forwarder_code, &creator_address, &party_b_forwarder_salt)?;

    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_address)?;
    PARTY_A_INTERCHAIN_ROUTER_ADDR.save(deps.storage, &party_a_interchain_router_address)?;
    PARTY_B_INTERCHAIN_ROUTER_ADDR.save(deps.storage, &party_b_interchain_router_address)?;
    COVENANT_INTERCHAIN_SPLITTER_ADDR.save(deps.storage, &splitter_address)?;
    PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &party_a_forwarder_address)?;
    PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_forwarder_address)?;
    COVENANT_SWAP_HOLDER_ADDR.save(deps.storage, &swap_holder_address)?;

    let preset_party_a_forwarder_fields = PresetIbcForwarderFields {
        remote_chain_connection_id: msg.party_a_config.party_chain_connection_id,
        remote_chain_channel_id: msg.party_a_config.party_to_host_chain_channel_id,
        denom: msg.party_a_config.native_denom,
        amount: msg.covenant_terms.party_a_amount,
        label: format!("{}_party_a_ibc_forwarder", msg.label),
        code_id: msg.contract_codes.ibc_forwarder_code,
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
    };
    let party_b_router_code = match msg.clone().party_b_config {
        CovenantPartyConfig::Interchain(_) => msg.contract_codes.interchain_router_code,
        CovenantPartyConfig::Native(_) => msg.contract_codes.native_router_code,
    };

    let covenant_denoms: Vec<String> = msg
        .splits
        .iter()
        .map(|split| split.denom.to_string())
        .collect();

    let preset_party_a_router_fields = PresetInterchainRouterFields {
        destination_chain_channel_id: msg.party_a_config.host_to_party_chain_channel_id,
        destination_receiver_addr: msg.party_a_config.party_receiver_addr,
        ibc_transfer_timeout: msg.party_a_config.ibc_transfer_timeout,
        label: format!("{}_party_a_interchain_router", msg.label),
        code_id: msg.contract_codes.interchain_router_code,
        denoms: covenant_denoms.clone(),
    };
    let preset_party_b_router_fields = PresetInterchainRouterFields {
        destination_chain_channel_id: msg.party_b_config.host_to_party_chain_channel_id,
        destination_receiver_addr: msg.party_b_config.party_receiver_addr,
        ibc_transfer_timeout: msg.party_b_config.ibc_transfer_timeout,
        label: format!("{}_party_b_interchain_router", msg.label),
        code_id: msg.contract_codes.interchain_router_code,
        denoms: covenant_denoms,
    };
    let preset_splitter_fields = PresetInterchainSplitterFields {
        splits: msg.splits,
        fallback_split: msg.fallback_split,
        label: format!("{}_interchain_splitter", msg.label),
        code_id: msg.contract_codes.splitter_code,
        party_a_addr: msg.party_a_config.get_final_receiver_address(),
        party_b_addr: msg.party_b_config.get_final_receiver_address(),
    };
    let preset_holder_fields = PresetSwapHolderFields {
        lockup_config: msg.lockup_config,
        parties_config: CovenantPartiesConfig {
            party_a: msg.party_a_config.to_covenant_party(),
            party_b: msg.party_b_config.to_covenant_party(),
        },
        covenant_terms: CovenantTerms::TokenSwap(msg.covenant_terms),
        code_id: msg.contract_codes.holder_code,
        label: format!("{}_swap_holder", msg.label),
    };
    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: vec![
            party_a_forwarder_address.to_string(),
            party_b_forwarder_address.to_string(),
            swap_holder_address.to_string(),
            party_a_interchain_router_address.to_string(),
            party_b_interchain_router_address.to_string(),
            splitter_address.to_string(),
        ],
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };

    let clock_instantiate2_msg = preset_clock_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        clock_salt
    )?;

    let holder_instantiate2_msg = preset_holder_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        holder_salt,
        clock_address.to_string(),
        splitter_address.to_string(),
    )?;

    let party_a_router_instantiate2_msg = preset_party_a_router_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_a_router_salt,
        clock_address.to_string(),
    )?;

    let party_b_router_instantiate2_msg = preset_party_b_router_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_b_router_salt,
        clock_address.to_string(),
    )?;

    let splitter_instantiate2_msg = preset_splitter_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        splitter_salt,
        clock_address.to_string(),
        party_a_interchain_router_address.to_string(),
        party_b_interchain_router_address.to_string(),
    )?;

    let party_a_ibc_forwarder_instantiate2_msg = preset_party_a_forwarder_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_a_forwarder_salt,
        clock_address.to_string(),
        swap_holder_address.to_string(),
    )?;

    let party_b_ibc_forwarder_instantiate2_msg = preset_party_b_forwarder_fields.to_instantiate2_msg(
        env.contract.address.to_string(),
        party_b_forwarder_salt,
        clock_address.to_string(),
        swap_holder_address.to_string(),
    )?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_messages(vec![
            clock_instantiate2_msg,
            holder_instantiate2_msg,
            party_a_ibc_forwarder_instantiate2_msg,
            party_b_ibc_forwarder_instantiate2_msg,
            party_a_router_instantiate2_msg,
            party_b_router_instantiate2_msg,
            splitter_instantiate2_msg,
        ])
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(
            &COVENANT_CLOCK_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(
            &COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::SplitterAddress {} => Ok(to_json_binary(
            &COVENANT_INTERCHAIN_SPLITTER_ADDR.may_load(deps.storage)?,
        )?),
        QueryMsg::InterchainRouterAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_ROUTER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_ROUTER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::IbcForwarderAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
        QueryMsg::PartyDepositAddress { party } => {
            // here depending on the party we query their ibc forwarder.
            // if it's present, we then query it for a deposit address
            // which should return the address of ICA on a remote chain.
            // if no ibc forwarder is saved, we return the holder.
            let resp = if party == "party_a" {
                match PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else if party == "party_b" {
                match PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)? {
                    Some(addr) => deps.querier.query_wasm_smart(
                        addr,
                        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
                    )?,
                    None => COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?,
                }
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_json_binary(&resp)?)
        }
    }
}
