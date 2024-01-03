use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CLOCK_ADDRESS, CONTRACT_STATE, COVENANT_TERMS, LOCKUP_CONFIG, NEXT_CONTRACT, PARTIES_CONFIG,
    },
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use covenant_clock::helpers::enqueue_msg;
use covenant_utils::CovenantTerms;
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:covenant-swap-holder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;

    if msg.lockup_config.is_expired(&env.block) {
        return Err(ContractError::Std(StdError::generic_err(
            "past lockup config",
        )));
    }
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    PARTIES_CONFIG.save(deps.storage, &msg.parties_config)?;
    COVENANT_TERMS.save(deps.storage, &msg.covenant_terms)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "swap_holder_instantiate")
        .add_attributes(msg.get_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    if info.sender != CLOCK_ADDRESS.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_forward(deps, env, info.sender),
        ContractState::Expired => try_refund(deps, env, info.sender),
        ContractState::Complete => {
            Ok(Response::default().add_attribute("contract_state", "completed"))
        }
    }
}

fn try_forward(mut deps: DepsMut, env: Env, clock_addr: Addr) -> Result<Response, ContractError> {
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    // check if covenant is expired
    if lockup_config.is_expired(&env.block) {
        CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;
        return Ok(Response::default()
            .add_attribute("method", "try_forward")
            .add_attribute("result", "covenant_expired")
            .add_attribute("contract_state", "expired"));
    }

    let parties = PARTIES_CONFIG.load(deps.storage)?;
    let CovenantTerms::TokenSwap(covenant_terms) = COVENANT_TERMS.load(deps.storage)?;

    let mut party_a_coin = deps
        .querier
        .query_balance(env.contract.address.clone(), parties.party_a.native_denom)?;
    let mut party_b_coin = deps
        .querier
        .query_balance(env.contract.address, parties.party_b.native_denom)?;

    if party_a_coin.amount < covenant_terms.party_a_amount {
        party_a_coin.amount = Uint128::zero();
    } else if party_b_coin.amount < covenant_terms.party_b_amount {
        party_b_coin.amount = Uint128::zero();
    }

    // if either of the coin amounts did not get updated to non-zero,
    // we are not ready for the swap yet
    if party_a_coin.amount.is_zero() || party_b_coin.amount.is_zero() {
        return Err(ContractError::InsufficientFunds {});
    }

    // otherwise we are ready to forward the funds to the next module
    let amount = vec![party_a_coin, party_b_coin];

    // first we query the deposit address of next module
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query = deps.querier.query_wasm_smart(
        next_contract,
        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
    )?;

    // if query returns None, then we error and wait
    let Some(deposit_address) = deposit_address_query else {
        return Err(ContractError::Std(StdError::not_found(
            "Next contract is not ready for receiving the funds yet",
        )));
    };

    let bank_msg = BankMsg::Send {
        to_address: deposit_address,
        amount,
    };

    let dequeue_msg = ContractState::complete_and_dequeue(deps.branch(), clock_addr.as_str())?;

    Ok(Response::default()
        .add_message(bank_msg)
        .add_message(dequeue_msg))
}

fn try_refund(mut deps: DepsMut, env: Env, clock_addr: Addr) -> Result<Response, ContractError> {
    let parties = PARTIES_CONFIG.load(deps.storage)?;

    // Query balance for the parties
    let party_a_coin = deps.querier.query_balance(
        env.contract.address.clone(),
        parties.party_a.native_denom.clone(),
    )?;
    let party_b_coin = deps
        .querier
        .query_balance(env.contract.address, parties.party_b.native_denom.clone())?;

    let messages = match (party_a_coin.amount.is_zero(), party_b_coin.amount.is_zero()) {
        // both balances being zero means that either:
        // 1. neither party deposited any funds in the first place
        // 2. we have refunded both parties
        // either way, this indicates completion
        (true, true) => {
            let msg = ContractState::complete_and_dequeue(deps.branch(), clock_addr.as_str())?;

            return Ok(Response::default()
                .add_message(msg)
                .add_attribute("method", "try_refund")
                .add_attribute("result", "nothing_to_refund")
                .add_attribute("contract_state", "complete"));
        }
        // party A failed to deposit. refund party B
        (true, false) => vec![parties
            .party_b
            .get_refund_msg(party_b_coin.amount, &env.block)],
        // party B failed to deposit. refund party A
        (false, true) => vec![parties
            .party_a
            .get_refund_msg(party_a_coin.amount, &env.block)],
        // not enough balances to perform the covenant swap.
        // refund denoms to both parties.
        (false, false) => vec![
            parties
                .party_a
                .get_refund_msg(party_a_coin.amount, &env.block),
            parties
                .party_b
                .get_refund_msg(party_b_coin.amount, &env.block),
        ],
    };

    Ok(Response::default()
        .add_attribute("method", "try_refund")
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::NextContract {} => Ok(to_json_binary(&NEXT_CONTRACT.may_load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_json_binary(&LOCKUP_CONFIG.may_load(deps.storage)?)?),
        QueryMsg::CovenantParties {} => {
            Ok(to_json_binary(&PARTIES_CONFIG.may_load(deps.storage)?)?)
        }
        QueryMsg::CovenantTerms {} => Ok(to_json_binary(&COVENANT_TERMS.may_load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        // the deposit address for swap-holder is the contract itself
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&Some(env.contract.address))?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            next_contract,
            lockup_config,
            parites_config,
            covenant_terms,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(addr) = clock_addr {
                let clock_address = deps.api.addr_validate(&addr)?;
                CLOCK_ADDRESS.save(deps.storage, &clock_address)?;
                resp = resp.add_attribute("clock_addr", addr);
            }

            if let Some(addr) = next_contract {
                let next_contract_addr = deps.api.addr_validate(&addr)?;
                NEXT_CONTRACT.save(deps.storage, &next_contract_addr)?;
                resp = resp.add_attribute("next_contract", addr);
            }

            if let Some(expiry_config) = lockup_config {
                if expiry_config.is_expired(&env.block) {
                    return Err(StdError::generic_err("lockup config is already past"));
                }
                LOCKUP_CONFIG.save(deps.storage, &expiry_config)?;
                resp = resp.add_attribute("lockup_config", expiry_config.to_string());
            }

            if let Some(parites_config) = *parites_config {
                PARTIES_CONFIG.save(deps.storage, &parites_config)?;
                resp = resp.add_attribute("parites_config", format!("{parites_config:?}"));
            }

            if let Some(covenant_terms) = covenant_terms {
                COVENANT_TERMS.save(deps.storage, &covenant_terms)?;
                resp = resp.add_attribute("covenant_terms", format!("{covenant_terms:?}"));
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => todo!(),
    }
}
