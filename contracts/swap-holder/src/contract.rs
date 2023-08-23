
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Coin, Uint128, CosmosMsg, BankMsg, StdError, IbcMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use crate::{msg::{InstantiateMsg, ExecuteMsg, ContractState}, state::{NEXT_CONTRACT, CLOCK_ADDRESS, LOCKUP_CONFIG, PARTIES_CONFIG, CONTRACT_STATE}, error::ContractError};

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

    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;

    // let parties_config = msg.parties_config.validate_config()?;
    let lockup_config = msg.lockup_config.validate(env.block)?;

    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, lockup_config)?;
    PARTIES_CONFIG.save(deps.storage, &msg.parties_config)?;

    Ok(Response::default()
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
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    let clock_addr = CLOCK_ADDRESS.load(deps.storage)?;
    if clock_addr != info.sender {
        return Err(ContractError::Unauthorized {})
    }

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_forward(deps, env, info),
        ContractState::Expired => try_refund(deps, env, info),
        ContractState::Complete => Ok(Response::default()
            .add_attribute("contract_state", "completed")
        ),
    }
}

fn try_forward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    // check if covenant is expired
    if lockup_config.is_due(env.block) {
        CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;
        return Ok(Response::default()
            .add_attribute("method", "try_forward")
            .add_attribute("result", "covenant_expired")
        )
    }

    let parties = PARTIES_CONFIG.load(deps.storage)?;

    let mut party_a_coin = Coin {
        denom: parties.party_a.provided_denom,
        amount: Uint128::zero(),
    };
    let mut party_b_coin = Coin {
        denom: parties.party_b.provided_denom,
        amount: Uint128::zero(),
    };

    // query holder balances
    let balances = deps.querier.query_all_balances(env.contract.address)?;
    // find the existing balances of covenant coins
    for coin in balances {
        if coin.denom == party_a_coin.denom 
        && coin.amount >= parties.party_a.amount {
            party_a_coin.amount = coin.amount;
        } else if coin.denom == party_a_coin.denom 
        && coin.amount >= parties.party_b.amount {
            party_b_coin.amount = coin.amount;
        }
    }

    // if either of the coin amounts did not get updated to non-zero,
    // we are not ready for the swap yet
    if party_a_coin.amount.is_zero() || party_b_coin.amount.is_zero() {
        return Err(ContractError::InsufficientFunds {})
    }

    // otherwise we are ready to forward the funds to the next module

    // first we query the deposit address of next module
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query = deps.querier.query_wasm_smart(
        next_contract,
        &covenant_utils::neutron_ica::QueryMsg::DepositAddress {},
    )?;
    // if query returns None, then we error and wait
    let Some(deposit_address) = deposit_address_query else {
        return Err(ContractError::Std(
            StdError::not_found("Next contract is not ready for receiving the funds yet")
        ))
    };

    let multi_send_msg = BankMsg::Send { 
        to_address: deposit_address,
        amount: vec![
            party_a_coin,
            party_b_coin,
        ]
    };

    // if bankMsg succeeds we can safely complete the holder
    CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;

    Ok(Response::default().add_message(CosmosMsg::Bank(multi_send_msg)))
}

fn try_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let parties = PARTIES_CONFIG.load(deps.storage)?;

    let mut party_a_coin = Coin {
        denom: parties.clone().party_a.provided_denom,
        amount: Uint128::zero(),
    };
    let mut party_b_coin = Coin {
        denom: parties.clone().party_b.provided_denom,
        amount: Uint128::zero(),
    };

    // query holder balances
    let balances = deps.querier.query_all_balances(env.contract.address)?;
    // find the existing balances of covenant coins
    for coin in balances {
        if coin.denom == party_a_coin.denom 
        && coin.amount >= parties.party_a.amount {
            party_a_coin.amount = coin.amount;
        } else if coin.denom == party_a_coin.denom 
        && coin.amount >= parties.party_b.amount {
            party_b_coin.amount = coin.amount;
        }
    }

    let messages = match (party_a_coin.amount.is_zero(), party_b_coin.amount.is_zero()) {
        // if both balances are zero, neither party deposited.
        // nothing to return, we complete.
        (true, true) => {
            CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
            return Ok(Response::default()
                .add_attribute("method", "try_refund")
                .add_attribute("result", "nothing_to_refund")
                .add_attribute("contract_state", "complete")
            )
        },
        // party A failed to deposit. refund party B
        (true, false) => {
            let refund_msg: IbcMsg = parties.party_b.get_ibc_refund_msg(party_b_coin.amount, env.block);
            vec![refund_msg]
        },
        // party B failed to deposit. refund party A
        (false, true) => {
            let refund_msg: IbcMsg = parties.party_a.get_ibc_refund_msg(party_a_coin.amount, env.block);
            vec![refund_msg]

        },
        // not enough balances to perform the covenant swap.
        // refund denoms to both parties.
        (false, false) => {
            let refund_b_msg: IbcMsg = parties.party_b.get_ibc_refund_msg(party_b_coin.amount, env.block.clone());
            let refund_a_msg: IbcMsg = parties.party_a.get_ibc_refund_msg(party_a_coin.amount, env.block);
            vec![refund_a_msg, refund_b_msg]
        },
    };

    CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;

    Ok(Response::default()
        .add_attribute("method", "try_refund")
        .add_messages(messages)
    )
}