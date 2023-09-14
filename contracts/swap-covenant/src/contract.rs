#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    SubMsg, WasmMsg, Reply, Deps, StdResult, Binary, Addr,
};

use cw2::set_contract_version;
use cw_utils::{parse_reply_instantiate_data, ParseReplyError};

use crate::{
    error::ContractError,
    state::{
        CLOCK_CODE, TIMEOUTS, COVENANT_CLOCK_ADDR, SWAP_HOLDER_CODE, PRESET_HOLDER_FIELDS, COVENANT_INTERCHAIN_SPLITTER_ADDR, PRESET_SPLITTER_FIELDS, COVENANT_SWAP_HOLDER_ADDR, IBC_FORWARDER_CODE, IBC_FEE, COVENANT_PARTIES, PARTY_A_IBC_FORWARDER_ADDR, PARTY_B_IBC_FORWARDER_ADDR, PARTY_A_INTERCHAIN_ROUTER_ADDR, INTERCHAIN_ROUTER_CODE, PARTY_B_INTERCHAIN_ROUTER_ADDR, INTERCHAIN_SPLITTER_CODE,
    }, msg::{InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = "crates.io:swap-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CLOCK_REPLY_ID: u64 = 1u64;
pub const PARTY_A_INTERCHAIN_ROUTER_REPLY_ID: u64 = 2u64;
pub const PARTY_B_INTERCHAIN_ROUTER_REPLY_ID: u64 = 3u64;
pub const SPLITTER_REPLY_ID: u64 = 4u64;
pub const SWAP_HOLDER_REPLY_ID: u64 = 5u64;
pub const PARTY_A_FORWARDER_REPLY_ID: u64 = 6u64;
pub const PARTY_B_FORWARDER_REPLY_ID: u64 = 7u64;


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
    INTERCHAIN_SPLITTER_CODE.save(deps.storage, &msg.splitter_code)?;
    INTERCHAIN_ROUTER_CODE.save(deps.storage, &msg.interchain_router_code)?;
    IBC_FORWARDER_CODE.save(deps.storage, &msg.ibc_forwarder_code)?;
    SWAP_HOLDER_CODE.save(deps.storage, &msg.preset_holder_fields.code_id)?;
    PRESET_SPLITTER_FIELDS.save(deps.storage, &msg.preset_splitter_fields)?;
    PRESET_HOLDER_FIELDS.save(deps.storage, &msg.preset_holder_fields)?;
    COVENANT_PARTIES.save(deps.storage, &msg.covenant_parties)?;
    TIMEOUTS.save(deps.storage, &msg.timeouts)?;
    IBC_FEE.save(deps.storage, &msg.preset_ibc_fee.to_ibc_fee())?;

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
        PARTY_A_INTERCHAIN_ROUTER_REPLY_ID => handle_party_a_interchain_router_reply(deps, env, msg),
        PARTY_B_INTERCHAIN_ROUTER_REPLY_ID => handle_party_b_interchain_router_reply(deps, env, msg),
        SPLITTER_REPLY_ID => handle_splitter_reply(deps, env, msg),
        SWAP_HOLDER_REPLY_ID => handle_swap_holder_reply(deps, env, msg),
        PARTY_A_FORWARDER_REPLY_ID => handle_party_a_ibc_forwarder_reply(deps, env, msg),
        PARTY_B_FORWARDER_REPLY_ID => handle_party_b_ibc_forwarder_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
    }
}

/// clock instantiation reply means we can proceed with the instantiation chain.
/// we store the clock address and submit the party A router instantiate tx.
pub fn handle_clock_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: clock reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the clock address
            let clock_addr = deps.api.addr_validate(&response.contract_address)?;
            COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;

            let code_id = INTERCHAIN_ROUTER_CODE.load(deps.storage)?;
            let party_config = COVENANT_PARTIES.load(deps.storage)?;
            
            let router_instantiate_msg = covenant_interchain_router::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                destination_chain_channel_id: party_config.party_a.host_to_party_chain_channel_id,
                destination_receiver_addr: party_config.party_a.party_receiver_addr.to_string(),
                ibc_transfer_timeout: party_config.party_a.ibc_transfer_timeout,
            };
            let party_a_router_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&router_instantiate_msg)?,
                funds: vec![],
                label: "party_a_router".to_string(),
            });

            Ok(Response::default()
                .add_attribute("method", "handle_clock_reply")
                .add_attribute("clock_addr", clock_addr)
                .add_attribute("router_code_id", code_id.to_string())
                .add_attribute("party_a_addr", party_config.party_a.addr)
                .add_attribute("router instantiate binary: ", to_binary(&router_instantiate_msg)?.to_base64())
                .add_submessage(
                    SubMsg::reply_always(party_a_router_instantiate_tx, PARTY_A_INTERCHAIN_ROUTER_REPLY_ID)
                )
            )
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "clock".to_string(),
            err,
        }),
    }
}


/// party A interchain router instantiation reply means we can proceed with the instantiation chain.
/// we store the instantiated router address and submit the party B router instantiation tx.
pub fn handle_party_a_interchain_router_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party A interchain router reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated router address
            let router_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_A_INTERCHAIN_ROUTER_ADDR.save(deps.storage, &router_addr)?;

            // load the fields relevant to router instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let code_id = INTERCHAIN_ROUTER_CODE.load(deps.storage)?;
            let party_config = COVENANT_PARTIES.load(deps.storage)?;

            let party_b_router_instantiate_tx: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(
                    &covenant_interchain_router::msg::InstantiateMsg {
                        clock_address: clock_addr.to_string(),
                        destination_chain_channel_id: party_config.party_b.host_to_party_chain_channel_id,
                        destination_receiver_addr: party_config.party_b.addr.to_string(),
                        ibc_transfer_timeout: party_config.party_b.ibc_transfer_timeout,
                    },
                )?,
                funds: vec![],
                label: "party b router".to_string(),
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_a_interchain_router_reply")
                .add_attribute("party_a_interchain_router_addr", router_addr)
                .add_submessage(
                    SubMsg::reply_always(party_b_router_instantiate_tx, PARTY_B_INTERCHAIN_ROUTER_REPLY_ID)
                )
            )
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party a router".to_string(),
            err,
        }),
    }
}

/// party B interchain router instantiation reply means we can proceed with the instantiation chain.
/// we store the instantiated router address and submit the interchain splitter instantiation tx.
pub fn handle_party_b_interchain_router_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: party B interchain router reply");

    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // validate and store the instantiated router address
            let router_addr = deps.api.addr_validate(&response.contract_address)?;
            PARTY_B_INTERCHAIN_ROUTER_ADDR.save(deps.storage, &router_addr)?;


            // load the fields relevant to splitter
            let code_id = INTERCHAIN_SPLITTER_CODE.load(deps.storage)?;
            let preset_splitter_fields = PRESET_SPLITTER_FIELDS.load(deps.storage)?;
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let party_a_router = PARTY_A_INTERCHAIN_ROUTER_ADDR.load(deps.storage)?;
            let swap_parties = COVENANT_PARTIES.load(deps.storage)?;

            let splitter_instantiate_msg = preset_splitter_fields.clone().to_instantiate_msg(
                clock_addr.to_string(),
                party_a_router.to_string(),
                swap_parties.party_a.addr.to_string(),
                router_addr.to_string(),
                swap_parties.party_b.addr.to_string(),
            ).map_err(|e| ContractError::ContractInstantiationError {
                contract: "splitter".to_string(),
                err: ParseReplyError::ParseFailure(e.to_string()),
            })?;

            let splitter_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id,
                msg: to_binary(&splitter_instantiate_msg)?,
                funds: vec![],
                label: preset_splitter_fields.label,
            });

            Ok(Response::default()
                .add_attribute("method", "handle_party_b_interchain_router_reply")
                .add_attribute("party_b_interchain_router_addr", router_addr)
                .add_submessage(SubMsg::reply_always(splitter_instantiate_tx, SPLITTER_REPLY_ID)))
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party b router".to_string(),
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
            let covenant_party = COVENANT_PARTIES.load(deps.storage)?.party_a;
            let covenant_terms = match PRESET_HOLDER_FIELDS.load(deps.storage)?.covenant_terms {
                covenant_utils::CovenantTerms::TokenSwap(terms) => terms,
            };

            let instantiate_msg = covenant_ibc_forwarder::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                next_contract: swap_holder_addr.to_string(),
                remote_chain_connection_id: covenant_party.party_chain_connection_id,
                remote_chain_channel_id: covenant_party.party_to_host_chain_channel_id,
                denom: covenant_party.native_denom,
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
            let covenant_party = COVENANT_PARTIES.load(deps.storage)?.party_b;
            let covenant_terms = match PRESET_HOLDER_FIELDS.load(deps.storage)?.covenant_terms {
                covenant_utils::CovenantTerms::TokenSwap(terms) => terms,
            };
            let swap_holder = COVENANT_SWAP_HOLDER_ADDR.load(deps.storage)?;

            let instantiate_msg = covenant_ibc_forwarder::msg::InstantiateMsg {
                clock_address: clock_addr.to_string(),
                next_contract: swap_holder.to_string(),
                remote_chain_connection_id: covenant_party.party_chain_connection_id,
                remote_chain_channel_id: covenant_party.party_to_host_chain_channel_id,
                denom: covenant_party.native_denom,
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
            contract: "party_a_forwarder".to_string(),
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
            let party_a_forwarder = PARTY_A_IBC_FORWARDER_ADDR.load(deps.storage)?;

            // load the fields relevant to ibc forwarder instantiation
            let clock_addr = COVENANT_CLOCK_ADDR.load(deps.storage)?;
            let clock_code_id = CLOCK_CODE.load(deps.storage)?;
            
            let swap_holder = COVENANT_SWAP_HOLDER_ADDR.load(deps.storage)?;

            let interchain_splitter = COVENANT_INTERCHAIN_SPLITTER_ADDR.load(deps.storage)?;
            let party_a_router = PARTY_A_INTERCHAIN_ROUTER_ADDR.load(deps.storage)?;
            let party_b_router = PARTY_B_INTERCHAIN_ROUTER_ADDR.load(deps.storage)?;


            let update_clock_whitelist_msg = WasmMsg::Migrate {
                contract_addr: clock_addr.to_string(),
                new_code_id: clock_code_id,
                msg: to_binary(&covenant_clock::msg::MigrateMsg::ManageWhitelist {
                    add: Some(vec![
                        party_a_forwarder.to_string(),
                        party_b_ibc_forwarder_addr.to_string(),
                        swap_holder.to_string(),
                        interchain_splitter.to_string(),
                        party_a_router.to_string(),
                        party_b_router.to_string(),
                    ]),
                    remove: None,
                })?,
            };

            Ok(Response::default()
                .add_attribute("method", "handle_party_b_ibc_forwarder_reply")
                .add_attribute("party_b_ibc_forwarder_addr", party_b_ibc_forwarder_addr)
                .add_message(update_clock_whitelist_msg)
        )
        }
        Err(err) => Err(ContractError::ContractInstantiationError {
            contract: "party_b ibc forwarder".to_string(),
            err,
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress{} => Ok(to_binary(&COVENANT_CLOCK_ADDR.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&COVENANT_SWAP_HOLDER_ADDR.may_load(deps.storage)?)?),
        QueryMsg::SplitterAddress {} =>  Ok(to_binary(&COVENANT_INTERCHAIN_SPLITTER_ADDR.may_load(deps.storage)?)?),
        QueryMsg::CovenantParties {} =>  Ok(to_binary(&COVENANT_PARTIES.may_load(deps.storage)?)?),
        QueryMsg::InterchainRouterAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_INTERCHAIN_ROUTER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_INTERCHAIN_ROUTER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_binary(&resp)?)
        },
        QueryMsg::IbcForwarderAddress { party } => {
            let resp = if party == "party_a" {
                PARTY_A_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else if party == "party_b" {
                PARTY_B_IBC_FORWARDER_ADDR.may_load(deps.storage)?
            } else {
                Some(Addr::unchecked("not found"))
            };
            Ok(to_binary(&resp)?)
        }
        QueryMsg::IbcFee {} => Ok(to_binary(&IBC_FEE.may_load(deps.storage)?)?),
        QueryMsg::Timeouts {} => Ok(to_binary(&TIMEOUTS.may_load(deps.storage)?)?),
    }
}
