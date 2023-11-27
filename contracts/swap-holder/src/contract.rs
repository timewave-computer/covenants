use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use covenant_utils::CovenantTerms;
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        CLOCK_ADDRESS, CONTRACT_STATE, COVENANT_TERMS, LOCKUP_CONFIG, NEXT_CONTRACT, PARTIES_CONFIG,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-swap-holder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const COMPLETION_REPLY_ID: u64 = 531;

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

    msg.lockup_config.validate(&env.block)?;

    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    PARTIES_CONFIG.save(deps.storage, &msg.parties_config)?;
    COVENANT_TERMS.save(deps.storage, &msg.covenant_terms)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    Ok(Response::default()
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
        ContractState::Instantiated => try_forward(deps, env),
        ContractState::Expired => try_refund(deps, env),
        ContractState::Complete => {
            Ok(Response::default().add_attribute("contract_state", "completed"))
        }
    }
}

fn try_forward(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    // check if covenant is expired
    if lockup_config.is_expired(env.block) {
        CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;
        return Ok(Response::default()
            .add_attribute("method", "try_forward")
            .add_attribute("result", "covenant_expired")
            .add_attribute("contract_state", "expired"));
    }

    let parties = PARTIES_CONFIG.load(deps.storage)?;
    let CovenantTerms::TokenSwap(covenant_terms) = COVENANT_TERMS.load(deps.storage)?;

    let mut party_a_coin = Coin {
        denom: parties.party_a.ibc_denom,
        amount: Uint128::zero(),
    };
    let mut party_b_coin = Coin {
        denom: parties.party_b.ibc_denom,
        amount: Uint128::zero(),
    };

    // query holder balances
    let balances = deps.querier.query_all_balances(env.contract.address)?;
    // find the existing balances of covenant coins
    for coin in balances {
        if coin.denom == party_a_coin.denom && coin.amount >= covenant_terms.party_a_amount {
            party_a_coin.amount = coin.amount;
        } else if coin.denom == party_b_coin.denom && coin.amount >= covenant_terms.party_b_amount {
            party_b_coin.amount = coin.amount;
        }
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

    let multi_send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: deposit_address,
        amount,
    });

    Ok(Response::default().add_submessage(SubMsg::reply_on_success(
        multi_send_msg,
        COMPLETION_REPLY_ID,
    )))
}

fn try_refund(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let parties = PARTIES_CONFIG.load(deps.storage)?;

    let mut party_a_coin = Coin {
        denom: parties.clone().party_a.ibc_denom,
        amount: Uint128::zero(),
    };
    let mut party_b_coin = Coin {
        denom: parties.clone().party_b.ibc_denom,
        amount: Uint128::zero(),
    };

    // query holder balances
    let balances = deps.querier.query_all_balances(env.contract.address)?;
    // find the existing balances of covenant coins
    for coin in balances {
        if coin.denom == party_a_coin.denom {
            party_a_coin.amount = coin.amount;
        } else if coin.denom == party_b_coin.denom {
            party_b_coin.amount = coin.amount;
        }
    }

    let messages = match (party_a_coin.amount.is_zero(), party_b_coin.amount.is_zero()) {
        // both balances being zero means that either:
        // 1. neither party deposited any funds in the first place
        // 2. we have refunded both parties
        // either way, this indicates completion
        (true, true) => {
            CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
            return Ok(Response::default()
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id == COMPLETION_REPLY_ID {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
        Ok(Response::default()
            .add_attribute("method", "reply_complete")
            .add_attribute("contract_state", "complete"))
    } else {
        Err(ContractError::UnexpectedReplyId {})
    }
}
