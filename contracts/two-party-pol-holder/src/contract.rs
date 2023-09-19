use astroport::pair::Cw20HookMsg;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Deps, StdResult, Binary, to_binary, CosmosMsg, WasmMsg, BankMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, BalanceResponse};

use crate::{msg::{InstantiateMsg, QueryMsg, ExecuteMsg, RagequitConfig, LockupConfig, ContractState}, state::{POOL_ADDRESS, NEXT_CONTRACT, CLOCK_ADDRESS, RAGEQUIT_CONFIG, LOCKUP_CONFIG, PARTIES_CONFIG, CONTRACT_STATE, DEPOSIT_DEADLINE, COVENANT_TERMS}, error::ContractError};

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
    let lockup_config = msg.lockup_config.validate(&env.block)?;
    match msg.deposit_deadline.clone() {
        Some(deadline) => {
            let validated_deadline = deadline.validate(&env.block)?;
            DEPOSIT_DEADLINE.save(deps.storage, validated_deadline)?;
        },
        None => {
            DEPOSIT_DEADLINE.save(deps.storage, &LockupConfig::None)?;
        }
    }

    POOL_ADDRESS.save(deps.storage, &pool_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, lockup_config)?;
    RAGEQUIT_CONFIG.save(deps.storage, &msg.ragequit_config)?;
    PARTIES_CONFIG.save(deps.storage, parties_config)?;
    COVENANT_TERMS.save(deps.storage, &msg.covenant_terms)?;

    Ok(Response::default()
        .add_attribute("method", "two_party_pol_holder_instantiate")
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

fn try_tick(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let state = CONTRACT_STATE.load(deps.storage)?;
    match state {
        ContractState::Instantiated => try_deposit(deps, env, info),
        ContractState::Active => check_expiration(deps, env, info),
        ContractState::Ragequit => todo!(),
        ContractState::Expired => todo!(),
        ContractState::Complete => Ok(Response::default().add_attribute("contract_state", "complete")),
    }
}

fn try_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    let parties = PARTIES_CONFIG.load(deps.storage)?;
    let terms = COVENANT_TERMS.load(deps.storage)?;

    // assert the balances
    let party_a_bal = deps.querier.query_balance(env.contract.address.to_string(), parties.party_a.provided_denom)?;
    let party_b_bal = deps.querier.query_balance(env.contract.address.to_string(), parties.party_b.provided_denom)?;

    if terms.party_a_amount < party_a_bal.amount || terms.party_b_amount < party_b_bal.amount {
        return Err(ContractError::InsufficientDeposits {})
    }
    
    // LiquidPooler is the next contract
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let msg = BankMsg::Send {
        to_address: next_contract.to_string(),
        amount: vec![party_a_bal, party_b_bal],
    };

    // advance the state to Active
    CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;

    Ok(Response::default()
        .add_attribute("method", "deposit_to_next_contract")
        .add_message(msg)
    )
}

fn check_expiration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;

    if !lockup_config.is_due(env.block) {
        return Ok(Response::default()
            .add_attribute("method", "check_expiration")
            .add_attribute("result", "not_due")
        )
    }

    let pool_address = POOL_ADDRESS.load(deps.storage)?;

    // We query the pool to get the contract for the pool info
    // The pool info is required to fetch the address of the
    // liquidity token contract. The liquidity tokens are CW20 tokens
    let pair_info: astroport::asset::PairInfo = deps.querier.query_wasm_smart(
        pool_address.to_string(),
        &astroport::pair::QueryMsg::Pair {},
    )?;

    // We query our own liquidity token balance
    let liquidity_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        pair_info.clone().liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    // We withdraw our liquidity constructing a CW20 send message
    // The message contains our liquidity token balance
    // The pool address and a message to call the withdraw liquidity hook of the pool contract
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool_address.to_string(),
        amount: liquidity_token_balance.balance,
        msg: to_binary(withdraw_liquidity_hook)?,
    };

    // advance state to Expired
    CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;

    // We execute the message on the liquidity token contract
    // This will burn the LP tokens and withdraw liquidity into the holder
    Ok(Response::default()
        .add_attribute("method", "check_expiration")
        .add_attribute("result", "expired")
        .add_attribute("lp_token_amount", liquidity_token_balance.balance)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: to_binary(withdraw_msg)?,
            funds: vec![],
        })))
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
    if liquidity_token_balance.balance.is_zero() {
        return Err(ContractError::NoLpTokensAvailable {})
    }
    
    // activate the ragequit in terms
    rq_terms.active = true;

    // apply the ragequit penalty
    let parties = parties.apply_ragequit_penalty(rq_party.clone(), rq_terms.penalty)?;
    let rq_party = parties.get_party_by_addr(rq_party.addr)?;
    
    // generate the withdraw_liquidity hook for the ragequitting party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool_address.to_string(),
        // take the ragequitting party share of the position
        amount: liquidity_token_balance.balance.checked_mul_floor(rq_party.share)
            .map_err(|_| ContractError::FractionMulError {})?,
        msg: to_binary(withdraw_liquidity_hook)?,
    };

    // update the state to reflect ragequit
    CONTRACT_STATE.save(deps.storage, &crate::msg::ContractState::Ragequit)?;

    // TODO: need some kind of state representation of pending withdrawals
    // to distinguish allocations of ragequitting party from the non-rq party
    
    Ok(Response::default()
        .add_attribute("method", "ragequit")
        .add_attribute("caller", rq_party.addr)
        .add_message(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_info.liquidity_token.to_string(),
                msg: to_binary(withdraw_msg)?,
                funds: vec![],
            })
        )
    )
}

fn try_claim(
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