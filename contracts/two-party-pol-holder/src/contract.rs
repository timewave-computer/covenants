use std::ops::Mul;

use astroport::{pair::Cw20HookMsg, DecimalCheckedOps, asset::Asset};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Deps, StdResult, Binary, to_binary, BankMsg, CosmosMsg, WasmMsg, Coin};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use covenant_utils::LockupConfig;
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg};

use crate::{msg::{InstantiateMsg, QueryMsg, ExecuteMsg, ContractState, RagequitConfig, RagequitState, TwoPartyPolCovenantConfig}, state::{NEXT_CONTRACT, CLOCK_ADDRESS, RAGEQUIT_CONFIG, LOCKUP_CONFIG, CONTRACT_STATE, DEPOSIT_DEADLINE, POOL_ADDRESS, PARTY_A_ROUTER, PARTY_B_ROUTER, COVENANT_CONFIG}, error::ContractError};

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
    let party_a_router = deps.api.addr_validate(&msg.party_a_router)?;
    let party_b_router = deps.api.addr_validate(&msg.party_b_router)?;

    POOL_ADDRESS.save(deps.storage, &pool_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    RAGEQUIT_CONFIG.save(deps.storage, &msg.ragequit_config)?;
    PARTY_A_ROUTER.save(deps.storage, &party_a_router)?;
    PARTY_B_ROUTER.save(deps.storage, &party_b_router)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    COVENANT_CONFIG.save(deps.storage, &msg.covenant_config)?;

    match &msg.deposit_deadline {
        Some(deadline) => {
            deadline.validate(&env.block)?;
            DEPOSIT_DEADLINE.save(deps.storage, deadline)?;
        },
        None => {
            DEPOSIT_DEADLINE.save(deps.storage, &LockupConfig::None)?;
        }
    }

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
    //     ExecuteMsg::Claim {} => try_claim(deps, env, info),
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        _ => Ok(Response::default()),
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
        ContractState::Active => check_expiration(deps, env),
        // ContractState::Ragequit => try_ragequit(deps, env, info),
        // ContractState::Expired => todo!(),
        ContractState::Complete => Ok(Response::default().add_attribute("contract_state", "complete")),
        _ => Ok(Response::default()),
    }
}

fn try_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = COVENANT_CONFIG.load(deps.storage)?;

    // assert the balances
    let party_a_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        config.party_a.party_contibution.denom)?;
    let party_b_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        config.party_b.party_contibution.denom)?;

    let deposit_deadline = DEPOSIT_DEADLINE.load(deps.storage)?;
    let party_a_fulfilled = config.party_a.party_contibution.amount < party_a_bal.amount;
    let party_b_fulfilled = config.party_b.party_contibution.amount < party_b_bal.amount;

    // note: even if both parties deposit their funds in time,
    // it is important to trigger this method before the expiry block
    // if deposit deadline is due we complete and refund
    if deposit_deadline.is_expired(env.block.clone()) {
        let a_router = PARTY_A_ROUTER.load(deps.storage)?;
        let b_router = PARTY_B_ROUTER.load(deps.storage)?;

        let refund_messages: Vec<CosmosMsg> = match (party_a_bal.amount.is_zero(), party_b_bal.amount.is_zero()) {
            // both balances empty, we complete
            (true, true) => {
                CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
                return Ok(Response::default()
                    .add_attribute("method", "try_deposit")
                    .add_attribute("state", "complete"))
            },
            // refund party B
            (true, false) => vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: b_router.to_string(),
                amount: vec![party_b_bal],
            })],
            // refund party A
            (false, true) => vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: a_router.to_string(),
                amount: vec![party_a_bal],
            })],
            // refund both
            (false, false) => vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: a_router.to_string(),
                    amount: vec![party_a_bal],
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: b_router.to_string(),
                    amount: vec![party_b_bal],
                }),
            ],
        };
        return Ok(Response::default()
            .add_attribute("method", "try_deposit")
            .add_attribute("action", "refund")
            .add_messages(refund_messages)
        )
    } else if !party_a_fulfilled || !party_b_fulfilled {
        // if deposit deadline is not yet due and both parties did not fulfill we error
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
) -> Result<Response, ContractError> {
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;

    if !lockup_config.is_expired(env.block) {
        return Ok(Response::default()
            .add_attribute("method", "check_expiration")
            .add_attribute("result", "not_due")
        )
    }

    // advance state to Expired to enable claims
    CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;

    Ok(Response::default()
        .add_attribute("method", "check_expiration")
        .add_attribute("contract_state", "expired")
    )
}

fn try_ragequit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // first we error out if ragequit is disabled
    let mut rq_config = match RAGEQUIT_CONFIG.load(deps.storage)? {
        RagequitConfig::Disabled => return Err(ContractError::RagequitDisabled {}),
        RagequitConfig::Enabled(terms) => terms,
    };
    let current_state = CONTRACT_STATE.load(deps.storage)?;
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    let mut covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let pool = POOL_ADDRESS.load(deps.storage)?;

    // ragequit is only possible when contract is in Active state.
    if current_state != ContractState::Active {
        return Err(ContractError::NotActive {})
    }
    // we also validate an edge case where it did expire but
    // did not receive a tick yet. tick is then required to advance.
    if lockup_config.is_expired(env.block) {
        return Err(ContractError::Expired {})
    }

    // authorize the message sender
    let (mut rq_party, mut counterparty) = covenant_config.authorize_sender(info.sender)?;
    // after all validations we are ready to perform the ragequit.
    // first we apply the ragequit penalty on both parties allocations
    rq_party.allocation -= rq_config.penalty;
    counterparty.allocation += rq_config.penalty;
    covenant_config.update_parties(rq_party.clone(), counterparty.clone());

    // We query the pool to get the contract for the pool info
    // The pool info is required to fetch the address of the
    // liquidity token contract. The liquidity tokens are CW20 tokens
    let pair_info: astroport::asset::PairInfo = deps
        .querier
        .query_wasm_smart(pool.to_string(), &astroport::pair::QueryMsg::Pair {})?;
    println!("pair info: {:?}", pair_info);

    // We query our own liquidity token balance
    let liquidity_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        pair_info.clone().liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    println!("liquidity_token_balance: {:?}", liquidity_token_balance);

    // if no lp tokens are available, no point to ragequit
    if liquidity_token_balance.balance.is_zero() {
        return Err(ContractError::NoLpTokensAvailable {})
    }
    
    // we figure out the amounts of underlying tokens that rq party would receive
    let rq_party_lp_token_amount = liquidity_token_balance.balance
        .checked_mul_floor(rq_party.allocation)
        .map_err(|_| ContractError::FractionMulError {})?;
    let rq_entitled_assets: Vec<Asset> = deps.querier
        .query_wasm_smart(
            pool.to_string(), 
            &astroport::pair::QueryMsg::Share { amount: rq_party_lp_token_amount },
        )?;
    println!("entitled assets: {:?}", rq_entitled_assets);
    // reflect the ragequit in ragequit config
    rq_config.state = Some(RagequitState::from_share_response(rq_entitled_assets, rq_party.clone())?);

    // generate the withdraw_liquidity hook for the ragequitting party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool.to_string(),
        amount: rq_party_lp_token_amount,
        msg: to_binary(withdraw_liquidity_hook)?,
    };

    // update the states
    RAGEQUIT_CONFIG.save(deps.storage, &RagequitConfig::Enabled(rq_config))?;
    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Ragequit)?;

    Ok(Response::default()
        .add_attribute("method", "ragequit")
        .add_attribute("caller", rq_party.party_addr)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: to_binary(withdraw_msg)?,
            funds: vec![],
        })
    ))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.load(deps.storage)?)?),
        QueryMsg::RagequitConfig {} => Ok(to_binary(&RAGEQUIT_CONFIG.load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_binary(&LOCKUP_CONFIG.load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.load(deps.storage)?)?),
        QueryMsg::NextContract {} => Ok(to_binary(&NEXT_CONTRACT.load(deps.storage)?)?),
        QueryMsg::PoolAddress {} => Ok(to_binary(&POOL_ADDRESS.load(deps.storage)?)?),
        QueryMsg::RouterPartyA {} => Ok(to_binary(&PARTY_A_ROUTER.load(deps.storage)?)?),
        QueryMsg::RouterPartyB {} => Ok(to_binary(&PARTY_B_ROUTER.load(deps.storage)?)?),
        QueryMsg::DepositDeadline {} => Ok(to_binary(&DEPOSIT_DEADLINE.load(deps.storage)?)?),
    }
}