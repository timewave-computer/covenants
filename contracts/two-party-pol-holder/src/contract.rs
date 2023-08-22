use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Deps, StdResult, Binary, to_binary, BalanceResponse};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use crate::{msg::{InstantiateMsg, QueryMsg, ExecuteMsg, RagequitConfig}, state::{POOL_ADDRESS, NEXT_CONTRACT, CLOCK_ADDRESS, RAGEQUIT_CONFIG, LOCKUP_CONFIG, PARTIES_CONFIG, CONTRACT_STATE}, error::ContractError};

const CONTRACT_NAME: &str = "crates.io:covenant-two-party-pol-holder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    deps.api.debug("WASMDEBUG: covenant-two-party-pol-holder instantiate");

    let pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;

    let parties_config = msg.parties_config.validate_config()?;
    let lockup_config = msg.lockup_config.validate(env.block)?;

    POOL_ADDRESS.save(deps.storage, &pool_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, lockup_config)?;
    RAGEQUIT_CONFIG.save(deps.storage, &msg.ragequit_config)?;
    PARTIES_CONFIG.save(deps.storage, parties_config)?;

    Ok(Response::default()
        .add_attributes(msg.get_response_attributes())
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Ragequit {} => try_ragequit(deps, env, info),
        ExecuteMsg::Claim {} => try_claim(deps, env, info),
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

fn try_ragequit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // if lockup period had passed, just claim the tokens instead of ragequitting
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    if lockup_config.is_due(env.block) {
        return Err(ContractError::RagequitWithLockupPassed {})
    } 
    
    // only the involved parties can initiate the ragequit
    let parties = PARTIES_CONFIG.load(deps.storage)?;
    let rq_party = parties.validate_caller(info.sender)?;

    let mut rq_terms = match RAGEQUIT_CONFIG.load(deps.storage)? {
        // if ragequit is not enabled for this covenant we error
        RagequitConfig::Disabled => return Err(ContractError::RagequitDisabled {}),
        RagequitConfig::Enabled(terms) => {
            if terms.active {
                return Err(ContractError::RagequitAlreadyActive {})
            }
            terms
        },
    };

    let pool_address = POOL_ADDRESS.load(deps.storage)?;

    // We query the pool to get the contract for the pool info
    // The pool info is required to fetch the address of the
    // liquidity token contract. The liquidity tokens are CW20 tokens
    let pair_info: astroport::asset::PairInfo = deps
        .querier
        .query_wasm_smart(pool_address.to_string(), &astroport::pair::QueryMsg::Pair {})?;

    // We query our own liquidity token balance
    let liquidity_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        pair_info.clone().liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    // if no lp tokens are available, no point to ragequit
    if liquidity_token_balance.amount.amount.is_zero() {
        return Err(ContractError::NoLpTokensAvailable {})
    }
    
    // activate the ragequit in terms
    rq_terms.active = true;

    // apply the ragequit penalty
    let parties = parties.apply_ragequit_penalty(rq_party, rq_terms.penalty)?;



    Ok(Response::default())
}

fn try_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    Ok(Response::default())
}

fn try_tick(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.load(deps.storage)?)?),
        QueryMsg::RagequitConfig {} => Ok(to_binary(&RAGEQUIT_CONFIG.load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_binary(&LOCKUP_CONFIG.load(deps.storage)?)?),
        QueryMsg::PartiesConfig {} => Ok(to_binary(&PARTIES_CONFIG.load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.load(deps.storage)?)?),
        QueryMsg::NextContract {} => Ok(to_binary(&NEXT_CONTRACT.load(deps.storage)?)?),
    }
}