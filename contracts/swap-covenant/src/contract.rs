#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    SubMsg, WasmMsg, Reply,
};

use covenant_utils::RefundConfig;
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{
        CLOCK_CODE, TIMEOUTS, COVENANT_CLOCK_ADDR, SWAP_HOLDER_CODE, PRESET_HOLDER_FIELDS, COVENANT_INTERCHAIN_SPLITTER_ADDR, INTECHAIN_SPLITTER_CODE, PRESET_SPLITTER_FIELDS, COVENANT_SWAP_HOLDER_ADDR, IBC_FORWARDER_CODE, IBC_FEE, COVENANT_PARTIES, COVENANT_TERMS, PARTY_A_IBC_FORWARDER_ADDR, PARTY_B_IBC_FORWARDER_ADDR,
    },
};

const CONTRACT_NAME: &str = "crates.io:swap-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const CLOCK_REPLY_ID: u64 = 1u64;
pub const SPLITTER_REPLY_ID: u64 = 2u64;
pub const SWAP_HOLDER_REPLY_ID: u64 = 3u64;
pub const PARTY_A_FORWARDER_REPLY_ID: u64 = 4u64;
pub const PARTY_B_FORWARDER_REPLY_ID: u64 = 5u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store all the codes for covenant configuration
    CLOCK_CODE.save(deps.storage, &msg.preset_clock_fields.clock_code)?;
    SWAP_HOLDER_CODE.save(deps.storage, &msg.preset_holder_fields.code_id)?;
    IBC_FORWARDER_CODE.save(deps.storage, &msg.ibc_forwarder_code)?;
    PRESET_HOLDER_FIELDS.save(deps.storage, &msg.preset_holder_fields)?;
    COVENANT_PARTIES.save(deps.storage, &msg.covenant_parties)?;
    COVENANT_TERMS.save(deps.storage, &msg.covenant_terms)?;
    TIMEOUTS.save(deps.storage, &msg.timeouts)?;

    // we start the module instantiation chain with the clock
    let clock_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: msg.preset_clock_fields.clock_code,
        msg: to_binary(&msg.preset_clock_fields.clone().to_instantiate_msg())?,
        funds: vec![],
        label: msg.preset_clock_fields.label,
    });

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_submessage(SubMsg::reply_on_success(
            clock_instantiate_tx,
            CLOCK_REPLY_ID,
        ))
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CLOCK_REPLY_ID => handle_clock_reply(deps, env, msg),
        SPLITTER_REPLY_ID => handle_splitter_reply(deps, env, msg),
        SWAP_HOLDER_REPLY_ID => handle_swap_holder_reply(deps, env, msg),
        PARTY_A_FORWARDER_REPLY_ID => handle_party_a_ibc_forwarder_reply(deps, env, msg),
        PARTY_B_FORWARDER_REPLY_ID => handle_party_b_ibc_forwarder_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
    }
}

/// clock instantiation reply means we can proceed with the instantiation chain.
/// we store the clock address and submit the splitter instantiate tx.
pub fn handle_clock_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: clock reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the clock address
            let clock_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;

            // load the fields relevant to splitter
            let code_id = INTECHAIN_SPLITTER_CODE.load(deps.storage)?;
            let preset_splitter_fields = PRESET_SPLITTER_FIELDS.load(deps.storage)?;

            let splitter_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(
                    &preset_splitter_fields
                        .clone()
                        .to_instantiate_msg(clock_addr.to_string()),
                )?,
                funds: vec![],
                label: preset_splitter_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_clock_reply")
                .add_attribute("clock_address", clock_addr)
                .add_submessage(SubMsg::reply_always(splitter_instantiate_tx, SPLITTER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "clock".to_string(),
            err,
        }),
    }
}

/// splitter instantiation reply means we can proceed with the instantiation chain.
/// we store the splitter address and submit the swap holder instantiate tx.
pub fn handle_splitter_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: splitter reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the splitter address
            let splitter_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_INTERCHAIN_SPLITTER_ADDR.save(deps.storage, &splitter_addr)?;

            // load the fields relevant to holder instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let code_id = SWAP_HOLDER_CODE.load(deps.storage)?;
            let preset_holder_fields = PRESET_HOLDER_FIELDS.load(deps.storage)?;

            let holder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(
                    &preset_holder_fields
                        .clone()
                        .to_instantiate_msg(clock_addr.to_string(), splitter_addr.to_string()),
                )?,
                funds: vec![],
                label: preset_holder_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_splitter_reply")
                .add_attribute("splitter_addr", splitter_addr)
                .add_submessage(SubMsg::reply_always(holder_instantiate_tx, SWAP_HOLDER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "splitter".to_string(),
            err,
        }),
    }
}

/// swap instantiation reply means we can proceed with the instantiation chain.
/// we store the swap holder address and submit the party A ibc forwarder instantiate tx.
pub fn handle_swap_holder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: swap holder reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the swap holder address
            let swap_holder_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_SWAP_HOLDER_ADDR.save(deps.storage, &swap_holder_addr)?;

            // load the fields relevant to ibc forwarder instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let code_id = IBC_FORWARDER_CODE.load(deps.storage)?;
            let timeouts = TIMEOUTS.load(deps.storage)?;
            let ibc_fee = IBC_FEE.load(deps.storage)?;
            let covenant_parties = COVENANT_PARTIES.load(deps.storage)?;
            let covenant_terms = COVENANT_TERMS.load(deps.storage)?;
            let refund_config = match covenant_parties.party_a.refund_config {
                RefundConfig::Ibc(r) => r,
                _ => return Err(ContractError::ContractInstantiationError { 
                    contract: "party_a_forwarder".to_string(),
                    err: cw_utils::ParseReplyError::ParseFailure("no remote chain info".to_string()),
                })
            };

            let instantiate_msg = covenant_ibc_forwarder::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                next_contract: swap_holder_addr.to_string(),
                remote_chain_connection_id: refund_config.connection_id,
                remote_chain_channel_id: refund_config.channel_id,
                denom: covenant_parties.party_a.provided_denom,
                amount: covenant_terms.party_a_amount,
                ibc_fee,
                ibc_transfer_timeout: timeouts.ibc_transfer_timeout,
                ica_timeout: timeouts.ica_timeout,
            };

            let party_a_forwarder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: "party_a_forwarder".to_string(),
            });

            Ok(Response::default()
                .add_attribute("method", "handle_swap_holder_reply")
                .add_attribute("swap_holder_addr", swap_holder_addr)
                .add_submessage(SubMsg::reply_always(party_a_forwarder_instantiate_tx, PARTY_A_FORWARDER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "swap holder".to_string(),
            err,
        }),
    }
}

/// party A ibc forwarder reply means we can proceed with the instantiation chain.
/// we store the party A ibc forwarder address and submit the party B ibc forwarder instantiate tx.
pub fn handle_party_a_ibc_forwarder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party A ibc forwader reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the party A ibc forwarder address
            let party_a_ibc_forwarder_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_A_IBC_FORWARDER_ADDR.save(deps.storage, &party_a_ibc_forwarder_addr)?;

            // load the fields relevant to ibc forwarder instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let code_id = IBC_FORWARDER_CODE.load(deps.storage)?;
            let timeouts = TIMEOUTS.load(deps.storage)?;
            let ibc_fee = IBC_FEE.load(deps.storage)?;
            let covenant_parties = COVENANT_PARTIES.load(deps.storage)?;
            let covenant_terms = COVENANT_TERMS.load(deps.storage)?;
            let swap_holder = COVENANT_SWAP_HOLDER_ADDR.load(deps.storage)?;

            let refund_config = match covenant_parties.party_b.refund_config {
                RefundConfig::Ibc(r) => r,
                _ => return Err(ContractError::ContractInstantiationError { 
                    contract: "party_b_forwarder".to_string(),
                    err: cw_utils::ParseReplyError::ParseFailure("no remote chain info".to_string()),
                })
            };

            let instantiate_msg = covenant_ibc_forwarder::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                next_contract: swap_holder.to_string(),
                remote_chain_connection_id: refund_config.connection_id,
                remote_chain_channel_id: refund_config.channel_id,
                denom: covenant_parties.party_b.provided_denom,
                amount: covenant_terms.party_b_amount,
                ibc_fee,
                ibc_transfer_timeout: timeouts.ibc_transfer_timeout,
                ica_timeout: timeouts.ica_timeout,
            };

            let party_b_forwarder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&instantiate_msg)?,
                funds: vec![],
                label: "party_b_forwarder".to_string(),
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_a_ibc_forwader")
                .add_attribute("party_a_ibc_forwarder_addr", party_a_ibc_forwarder_addr)
                .add_submessage(SubMsg::reply_always(party_b_forwarder_instantiate_tx, PARTY_B_FORWARDER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "swap holder".to_string(),
            err,
        }),
    }
}


/// party B ibc forwarder reply means that we instantiated all the contracts.
/// we store the party B ibc forwarder address and whitelist the contracts on our clock.
pub fn handle_party_b_ibc_forwarder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party B ibc forwader reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the party b ibc forwarder address
            let party_b_ibc_forwarder_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_B_IBC_FORWARDER_ADDR.save(deps.storage, &party_b_ibc_forwarder_addr)?;

            // load the fields relevant to ibc forwarder instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let clock_code_id = CLOCK_CODE.load(deps.storage)?;
            
            let swap_holder = COVENANT_SWAP_HOLDER_ADDR.load(deps.storage)?;
            let party_a_forwarder = PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?;

            let interchain_splitter = COVENANT_INTERCHAIN_SPLITTER_ADDR.load(deps.storage)?;


            let update_clock_whitelist_msg = WasmMsg::Migrate {
                contract_addr: clock_addr.to_string(),
                new_code_id: clock_code_id,
                msg: to_binary(&covenant_clock::msg::MigrateMsg::ManageWhitelist {
                    add: Some(vec![
                        party_a_forwarder.to_string(),
                        party_b_ibc_forwarder_addr.to_string(),
                        swap_holder.to_string(),
                        interchain_splitter.to_string(),
                    ]),
                    remove: None,
                })?,
            };

            Ok(Response::default()
                .add_attribute("method", "handle_party_a_ibc_forwader")
                .add_attribute("party_b_ibc_forwarder_addr", party_b_ibc_forwarder_addr)
                .add_message(update_clock_whitelist_msg))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "swap holder".to_string(),
            err,
        }),
    }
}
