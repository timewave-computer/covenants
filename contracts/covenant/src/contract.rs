
use cosmwasm_std::{DepsMut, Deps, Env, Response, StdResult, MessageInfo, Binary, to_binary, SubMsg, CosmosMsg, WasmMsg, Reply, SubMsgResult, StdError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

// use covenant_ls::msg::InstantiateMsg as LsInstantiateMsg;
// use covenant_depositor::msg::InstantiateMsg as DepositorInstantiateMsg;
// use covenant_lp::msg::InstantiateMsg as LpInstantiateMsg;
// use covenant_clock::msg::InstantiateMsg as ClockInstantiateMsg;
// use covenant_holder::msg::InstantiateMsg as HolderInstantiateMsg;


use crate::{msg::{QueryMsg, MigrateMsg, ExecuteMsg, InstantiateMsg}, error::ContractError, state::{LS_INSTANTIATION_DATA, CLOCK_INSTANTIATION_DATA, LP_INSTANTIATION_DATA, DEPOSITOR_INSTANTIATION_DATA, COVENANT_CLOCK, COVENANT_DEPOSITOR, COVENANT_LP, COVENANT_LS, LP_CODE, HOLDER_CODE, DEPOSITOR_CODE, LS_CODE}};

const CONTRACT_NAME: &str = "crates.io:covenant-covenant";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CLOCK_REPLY_ID: u64 = 1u64;
const HOLDER_REPLY_ID: u64 = 2u64;
const LP_REPLY_ID: u64 = 3u64;
const LS_REPLY_ID: u64 = 4u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    LP_CODE.save(deps.storage, &msg.lp_code)?;
    DEPOSITOR_CODE.save(deps.storage, &msg.depositor_code)?;
    LS_CODE.save(deps.storage, &msg.ls_code)?;
    HOLDER_CODE.save(deps.storage, &msg.holder_code)?;

    CLOCK_INSTANTIATION_DATA.save(deps.storage, &msg.clock_instantiate.clone())?;
    LP_INSTANTIATION_DATA.save(deps.storage, &msg.lp_instantiate)?;
    LS_INSTANTIATION_DATA.save(deps.storage, &msg.ls_instantiate)?;
    DEPOSITOR_INSTANTIATION_DATA.save(deps.storage, &msg.depositor_instantiate)?;

    let clock_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate { 
        admin: Some(env.contract.address.to_string()),
        code_id: msg.clock_code,
        msg: to_binary(&msg.clock_instantiate)?,
        funds: vec![],
        label: "covenant-clock".to_string(),
    });

    // instantiate clock first
    Ok(Response::default().add_submessage(
        SubMsg { 
            id: CLOCK_REPLY_ID,
            msg: clock_instantiate_tx,
            gas_limit: None,
            reply_on: cosmwasm_std::ReplyOn::Always,
        }
    ))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
    QueryMsg::DepositorAddress {  } => Ok(to_binary(&COVENANT_DEPOSITOR.may_load(deps.storage)?)?),
    QueryMsg::ClockAddress {  } => Ok(to_binary(&COVENANT_CLOCK.may_load(deps.storage)?)?),
    QueryMsg::LpAddress {  } => Ok(to_binary(&COVENANT_LP.may_load(deps.storage)?)?),
    QueryMsg::LsAddress {  } => Ok(to_binary(&COVENANT_LS.may_load(deps.storage)?)?),
}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");

    match msg.id {
        CLOCK_REPLY_ID => handle_clock_reply(deps, env, msg),
        HOLDER_REPLY_ID => handle_holder_reply(deps, env, msg),
        LP_REPLY_ID => handle_lp_reply(deps, env, msg),
        LS_REPLY_ID => handle_ls_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
    }
}

pub fn handle_clock_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let parsed_data = parse_reply_instantiate_data(msg);
    match parsed_data {
        Ok(response) => {
            // successful clock instantiation means we are ready to proceed with
            // remaining instantiations
            COVENANT_CLOCK.save(deps.storage, &response.contract_address)?;
            
            // let holder_data = HOLDER_INSTANTIATION_DATA.load(deps.storage)?;
            let holder_code = HOLDER_CODE.load(deps.storage)?;

            let holder_instantiate_tx = CosmosMsg::Wasm(WasmMsg::Instantiate { 
                admin: Some(env.contract.address.to_string()),
                code_id: holder_code,
                msg: to_binary(&true)?,
                funds: vec![],
                label: "covenant-holder".to_string(),
            });

            Ok(Response::default().add_submessage(
                SubMsg {
                    id: HOLDER_REPLY_ID,
                    msg: holder_instantiate_tx,
                    gas_limit: None,
                    reply_on: cosmwasm_std::ReplyOn::Always,
                }
            ))
        },
        Err(err) => Err(ContractError::Std(StdError::GenericErr { msg: err.to_string() })),
    }
}

pub fn handle_holder_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {

    Ok(Response::default())
}

pub fn handle_lp_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {

    Ok(Response::default())
}

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {

    Ok(Response::default())
}

