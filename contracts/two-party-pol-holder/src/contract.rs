use std::cmp::Ordering;
use std::collections::BTreeMap;

use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use covenant_utils::clock::{enqueue_msg, verify_clock};
use covenant_utils::split::SplitConfig;
use covenant_utils::withdraw_lp_helper::{generate_withdraw_msg, EMERGENCY_COMMITTEE_ADDR};
use cw2::set_contract_version;

use crate::msg::CovenantType;
use crate::state::{WithdrawState, LIQUID_POOLER_ADDRESS, WITHDRAW_STATE};
use crate::{
    error::ContractError,
    msg::{
        ContractState, DenomSplits, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
        RagequitConfig, RagequitState, TwoPartyPolCovenantConfig, TwoPartyPolCovenantParty,
    },
    state::{
        CLOCK_ADDRESS, CONTRACT_STATE, COVENANT_CONFIG, DENOM_SPLITS, DEPOSIT_DEADLINE,
        LOCKUP_CONFIG, RAGEQUIT_CONFIG,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
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

    // ensure that the deposit deadline is in the future
    ensure!(
        !msg.deposit_deadline.is_expired(&env.block),
        ContractError::DepositDeadlineValidationError {}
    );

    // validate that lockup expiration is after the deposit deadline
    match msg.deposit_deadline.partial_cmp(&msg.lockup_config) {
        Some(ordering) => ensure!(
            ordering == Ordering::Less,
            ContractError::LockupValidationError {}
        ),
        // we validate incompatible expirations
        None => return Err(ContractError::ExpirationValidationError {}),
    };

    if let Some(addr) = &msg.emergency_committee_addr {
        let committee_addr = deps.api.addr_validate(addr)?;
        EMERGENCY_COMMITTEE_ADDR.save(deps.storage, &committee_addr)?;
    }

    msg.covenant_config.validate(deps.api)?;
    msg.ragequit_config.validate(
        msg.covenant_config.party_a.allocation,
        msg.covenant_config.party_b.allocation,
    )?;

    // validate the splits and collect them into map
    let explicit_splits: BTreeMap<String, SplitConfig> = msg
        .splits
        .iter()
        .filter_map(|(denom, split)| {
            split
                .validate(
                    &msg.covenant_config.party_a.router,
                    &msg.covenant_config.party_b.router,
                )
                .ok()?;
            Some((denom.to_string(), split.to_owned()))
        })
        .collect();

    msg.fallback_split
        .as_ref()
        .map(|split_config| {
            split_config.validate(
                &msg.covenant_config.party_a.router,
                &msg.covenant_config.party_b.router,
            )
        })
        .transpose()?;

    DENOM_SPLITS.save(
        deps.storage,
        &DenomSplits {
            explicit_splits,
            fallback_split: msg.fallback_split.clone(),
        },
    )?;
    LIQUID_POOLER_ADDRESS.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    RAGEQUIT_CONFIG.save(deps.storage, &msg.ragequit_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    COVENANT_CONFIG.save(deps.storage, &msg.covenant_config)?;
    DEPOSIT_DEADLINE.save(deps.storage, &msg.deposit_deadline)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "two_party_pol_holder_instantiate")
        .add_attributes(msg.get_response_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match (CONTRACT_STATE.load(deps.storage)?, msg) {
        // incoming ticks when in instantiated state try to deposit the funds
        (ContractState::Instantiated, ExecuteMsg::Tick {}) => try_deposit(deps, env, info),
        // incoming ticks when in active state are used to check for expiration
        (ContractState::Active, ExecuteMsg::Tick {}) => check_expiration(deps, env, info),
        // incoming ticks when in expired state are no-ops
        (ContractState::Expired, ExecuteMsg::Tick {}) => Ok(Response::default()
            .add_attribute("method", "tick")
            .add_attribute("contract_state", "expired")),
        // incoming ticks when in processing ragequit state are no-ops
        (ContractState::Ragequit, ExecuteMsg::Tick {}) => Ok(Response::default()
            .add_attribute("method", "tick")
            .add_attribute("contract_state", "ragequit")),
        // incoming ticks when in completed state are used to refund the parties
        (ContractState::Complete, ExecuteMsg::Tick {}) => try_refund(deps, env, info),
        // ragequit is state-independent
        (current_state, ExecuteMsg::Ragequit {}) => try_ragequit(deps, env, info, current_state),
        // emergency withdraw is state-independent
        (_, ExecuteMsg::EmergencyWithdraw {}) => try_emergency_withdraw(deps, info),
        // claims can only be performed from ragequit or expired state
        (ContractState::Ragequit | ContractState::Expired, ExecuteMsg::Claim {}) => {
            try_claim(deps, info)
        }
        (_, ExecuteMsg::Claim {}) => Err(ContractError::ClaimError {}),
        // receiving distribute callback is state-independent
        (_, ExecuteMsg::Distribute {}) => try_distribute(deps, info),
        // receiving withdraw failed callback is state-independent
        (_, ExecuteMsg::WithdrawFailed {}) => try_withdraw_failed(deps, info),
        // distributing fallback splits is state-independent
        (_, ExecuteMsg::DistributeFallbackSplit { denoms }) => {
            try_distribute_fallback_split(deps, env, denoms)
        }
    }
}

fn try_distribute_fallback_split(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut available_balances = Vec::with_capacity(denoms.len());
    let denom_splits = DENOM_SPLITS.load(deps.storage)?;
    let contract_addr = env.contract.address.to_string();
    for denom in denoms {
        ensure!(
            !denom_splits.explicit_splits.contains_key(&denom),
            ContractError::UnauthorizedDenomDistribution {}
        );
        let queried_coin = deps.querier.query_balance(&contract_addr, denom)?;
        available_balances.push(queried_coin);
    }

    let fallback_distribution_messages =
        denom_splits.get_fallback_distribution_messages(available_balances);

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback_split")
        .add_messages(fallback_distribution_messages))
}

/// On claim, we should simply ask the LPer to withdraw the liquidity and execute a Distribute msg on the holder
fn try_claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    if WITHDRAW_STATE.load(deps.storage).is_ok() {
        return Err(ContractError::WithdrawAlreadyStarted {});
    }

    let covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let (claim_party, counterparty) = covenant_config.authorize_sender(info.sender.to_string())?;

    // if both parties already claimed everything we complete early
    if claim_party.allocation.is_zero() && counterparty.allocation.is_zero() {
        let clock_address = CLOCK_ADDRESS.load(deps.storage)?;
        let dequeue_message = ContractState::complete_and_dequeue(deps, clock_address.as_str())?;

        return Ok(Response::default()
            .add_attribute("method", "try_claim")
            .add_attribute("contract_state", "complete")
            .add_message(dequeue_message));
    }

    // set WithdrawState to include original data
    WITHDRAW_STATE.save(
        deps.storage,
        &WithdrawState::Processing {
            claimer_addr: claim_party.host_addr,
        },
    )?;

    // If type is share we only withdraw the claim party allocation
    // if type is side, we withdraw 100% of funds
    let withdraw_percentage = match covenant_config.covenant_type {
        CovenantType::Share => Some(claim_party.allocation),
        CovenantType::Side => None, // 100%
    };

    let lper = LIQUID_POOLER_ADDRESS.load(deps.storage)?;
    let withdraw_msg = generate_withdraw_msg(lper.to_string(), withdraw_percentage)?;

    Ok(Response::default().add_message(withdraw_msg))
}

fn try_emergency_withdraw(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    ensure!(
        !WITHDRAW_STATE.exists(deps.storage),
        ContractError::WithdrawAlreadyStarted {}
    );

    ensure!(
        info.sender == EMERGENCY_COMMITTEE_ADDR.load(deps.storage)?,
        ContractError::Unauthorized {}
    );

    WITHDRAW_STATE.save(deps.storage, &WithdrawState::Emergency {})?;

    let lper = LIQUID_POOLER_ADDRESS.load(deps.storage)?;
    let withdraw_msg = generate_withdraw_msg(lper.to_string(), None)?;

    Ok(Response::default().add_message(withdraw_msg))
}

fn try_distribute(mut deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Only pooler can call this
    ensure!(
        info.sender == LIQUID_POOLER_ADDRESS.load(deps.storage)?,
        ContractError::Unauthorized {}
    );

    let withdraw_state = WITHDRAW_STATE
        .load(deps.storage)
        .map_err(|_| ContractError::WithdrawStateNotStarted {})?;

    let covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let denom_splits = DENOM_SPLITS.load(deps.storage)?;

    let (claim_party, counterparty, denom_splits, is_rq) = match withdraw_state {
        WithdrawState::Processing { claimer_addr } => {
            let (claim_party, counterparty) = covenant_config.authorize_sender(claimer_addr)?;

            (claim_party, counterparty, denom_splits, false)
        }
        WithdrawState::ProcessingRagequit {
            claimer_addr,
            terms,
        } => {
            let (rq_party, counterparty) = covenant_config.authorize_sender(claimer_addr)?;
            let new_denom_split =
                denom_splits.apply_penalty(terms.penalty, &rq_party, &counterparty)?;

            (rq_party, counterparty, new_denom_split, true)
        }
        WithdrawState::Emergency {} => {
            return try_claim_side_based(
                deps,
                covenant_config.party_a.clone(),
                covenant_config.party_b.clone(),
                info.funds,
                covenant_config,
                denom_splits,
            )
        }
    };

    WITHDRAW_STATE.remove(deps.storage);

    match covenant_config.covenant_type {
        CovenantType::Share => {
            if is_rq {
                apply_rq_state_share(deps.branch(), claim_party.clone(), info.funds.clone())?;
            }

            try_claim_share_based(
                deps,
                claim_party,
                counterparty,
                info.funds,
                covenant_config,
                denom_splits,
            )
        }
        CovenantType::Side => {
            if is_rq {
                apply_rq_state_side(deps.branch(), claim_party.clone(), info.funds.clone())?;
            }

            try_claim_side_based(
                deps,
                claim_party,
                counterparty,
                info.funds,
                covenant_config,
                denom_splits,
            )
        }
    }
}

/// We don't do much on failed withdraw, as nothing changed so far.
/// We only change state on distribute msg.
fn try_withdraw_failed(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Assert the caller is the pooler
    let pooler_addr = LIQUID_POOLER_ADDRESS.load(deps.storage)?;
    ensure!(info.sender == pooler_addr, ContractError::Unauthorized {});

    WITHDRAW_STATE.remove(deps.storage);

    Ok(Response::default())
}

#[allow(clippy::too_many_arguments)]
fn try_claim_share_based(
    mut deps: DepsMut,
    mut claim_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    funds: Vec<Coin>,
    mut covenant_config: TwoPartyPolCovenantConfig,
    denom_splits: DenomSplits,
) -> Result<Response, ContractError> {
    let mut messages = denom_splits
        .get_single_receiver_distribution_messages(funds, claim_party.router.to_string());

    claim_party.allocation = Decimal::zero();

    // if other party had not claimed yet, we assign it the full position
    if !counterparty.allocation.is_zero() {
        counterparty.allocation = Decimal::one();
    } else {
        // otherwise both parties claimed everything and we can complete
        let clock_address = CLOCK_ADDRESS.load(deps.storage)?;
        let dequeue_message =
            ContractState::complete_and_dequeue(deps.branch(), clock_address.as_str())?;
        messages.push(dequeue_message.into());
    };

    covenant_config.update_parties(claim_party, counterparty);

    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;

    Ok(Response::default()
        .add_attribute("method", "claim_share_based")
        .add_messages(messages))
}

#[allow(clippy::too_many_arguments)]
fn try_claim_side_based(
    deps: DepsMut,
    mut claim_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    funds: Vec<Coin>,
    mut covenant_config: TwoPartyPolCovenantConfig,
    denom_splits: DenomSplits,
) -> Result<Response, ContractError> {
    let messages: Vec<CosmosMsg> = denom_splits.get_shared_distribution_messages(funds);

    claim_party.allocation = Decimal::zero();
    counterparty.allocation = Decimal::zero();
    covenant_config.update_parties(claim_party, counterparty);

    // update the states and dequeue from the clock
    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;
    let clock_address = CLOCK_ADDRESS.load(deps.storage)?;
    let dequeue_message = ContractState::complete_and_dequeue(deps, clock_address.as_str())?;

    Ok(Response::default()
        .add_attribute("method", "claim_side_based")
        .add_messages(messages)
        .add_message(dequeue_message))
}

/// attempts to route any available covenant party contribution denoms to
/// the parties that were responsible for contributing that denom.
fn try_refund(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)
        .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;

    let config = COVENANT_CONFIG.load(deps.storage)?;
    let contract_addr = env.contract.address.to_string();

    // assert the balances
    let party_a_bal = deps
        .querier
        .query_balance(&contract_addr, config.party_a.contribution.denom)?;
    let party_b_bal = deps
        .querier
        .query_balance(&contract_addr, config.party_b.contribution.denom)?;

    let refund_messages: Vec<CosmosMsg> = [
        (party_a_bal, config.party_a.router),
        (party_b_bal, config.party_b.router),
    ]
    .into_iter()
    // find the parties that need to be refunded
    .filter(|(party_coin, _)| !party_coin.amount.is_zero())
    // get the bank transfer of the party's contribution to the respective router
    .map(|(party_coin, to_address)| {
        BankMsg::Send {
            to_address,
            amount: vec![party_coin],
        }
        .into()
    })
    .collect();

    Ok(Response::default()
        .add_attribute("contract_state", "complete")
        .add_attribute("method", "try_refund")
        .add_messages(refund_messages))
}

fn try_deposit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)
        .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;

    let deposit_deadline = DEPOSIT_DEADLINE.load(deps.storage)?;
    if deposit_deadline.is_expired(&env.block) {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
        return Ok(Response::default()
            .add_attribute("method", "try_deposit")
            .add_attribute("deposit_deadline", "expired")
            .add_attribute("action", "complete"));
    }

    let config = COVENANT_CONFIG.load(deps.storage)?;
    let contract_addr = env.contract.address.to_string();

    // assert the balances
    let party_a_bal = deps
        .querier
        .query_balance(&contract_addr, config.party_a.contribution.denom)?;
    let party_b_bal = deps
        .querier
        .query_balance(&contract_addr, config.party_b.contribution.denom)?;

    let party_a_fulfilled = config.party_a.contribution.amount <= party_a_bal.amount;
    let party_b_fulfilled = config.party_b.contribution.amount <= party_b_bal.amount;

    // if either party did not fulfill their deposit, we error out
    ensure!(
        party_a_fulfilled && party_b_fulfilled,
        ContractError::InsufficientDeposits {}
    );

    // LiquidPooler is the next contract
    let liquid_pooler = LIQUID_POOLER_ADDRESS.load(deps.storage)?;
    let msg = BankMsg::Send {
        to_address: liquid_pooler.to_string(),
        amount: vec![party_a_bal, party_b_bal],
    };

    // advance the state to Active
    CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;

    Ok(Response::default()
        .add_attribute("method", "deposit_to_next_contract")
        .add_message(msg))
}

fn check_expiration(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)
        .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;

    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;

    if !lockup_config.is_expired(&env.block) {
        return Ok(Response::default()
            .add_attribute("method", "check_expiration")
            .add_attribute("result", "not_due"));
    }

    // advance state to Expired to enable claims
    CONTRACT_STATE.save(deps.storage, &ContractState::Expired)?;

    Ok(Response::default()
        .add_attribute("method", "check_expiration")
        .add_attribute("contract_state", "expired"))
}

fn try_ragequit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    current_state: ContractState,
) -> Result<Response, ContractError> {
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    let covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let lper = LIQUID_POOLER_ADDRESS.load(deps.storage)?;

    // first we error out if ragequit is disabled
    let rq_terms = match RAGEQUIT_CONFIG.load(deps.storage)? {
        RagequitConfig::Disabled => return Err(ContractError::RagequitDisabled {}),
        RagequitConfig::Enabled(terms) => terms,
    };

    // ragequit is only possible when contract is in Active state.
    ensure!(
        current_state == ContractState::Active,
        ContractError::NotActive {}
    );

    if WITHDRAW_STATE.load(deps.storage).is_ok() {
        return Err(ContractError::WithdrawAlreadyStarted {});
    }

    // we also validate an edge case where it did expire but
    // did not receive a tick yet. tick is then required to advance.
    if lockup_config.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // authorize the message sender
    let (rq_party, _) = covenant_config.authorize_sender(info.sender.to_string())?;

    // If type is share we only withdraw the claim party allocation
    // if type is side, we withdraw 100% of funds
    let withdraw_percentage = match covenant_config.covenant_type {
        CovenantType::Share => Some(rq_party.allocation - rq_terms.penalty),
        CovenantType::Side => None, // 100%
    };

    // set WithdrawState to include original data
    WITHDRAW_STATE.save(
        deps.storage,
        &WithdrawState::ProcessingRagequit {
            claimer_addr: rq_party.host_addr,
            terms: rq_terms,
        },
    )?;

    let withdraw_msg = generate_withdraw_msg(lper.to_string(), withdraw_percentage)?;

    Ok(Response::default().add_message(withdraw_msg))
}

pub fn apply_rq_state_side(
    deps: DepsMut,
    rq_party: TwoPartyPolCovenantParty,
    coins: Vec<Coin>,
) -> Result<(), ContractError> {
    if let RagequitConfig::Enabled(mut rq_terms) = RAGEQUIT_CONFIG.load(deps.storage)? {
        rq_terms.state = Some(RagequitState { coins, rq_party });

        RAGEQUIT_CONFIG.save(deps.storage, &RagequitConfig::Enabled(rq_terms))?;
    }
    Ok(())
}

pub fn apply_rq_state_share(
    deps: DepsMut,
    rq_party: TwoPartyPolCovenantParty,
    coins: Vec<Coin>,
) -> Result<(), ContractError> {
    if let RagequitConfig::Enabled(mut rq_terms) = RAGEQUIT_CONFIG.load(deps.storage)? {
        rq_terms.state = Some(RagequitState { coins, rq_party });

        RAGEQUIT_CONFIG.save(deps.storage, &RagequitConfig::Enabled(rq_terms))?;
    };

    CONTRACT_STATE.save(deps.storage, &ContractState::Ragequit)?;

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.load(deps.storage)?)?),
        QueryMsg::RagequitConfig {} => Ok(to_json_binary(&RAGEQUIT_CONFIG.load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_json_binary(&LOCKUP_CONFIG.load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.load(deps.storage)?)?),
        QueryMsg::NextContract {} => {
            Ok(to_json_binary(&LIQUID_POOLER_ADDRESS.load(deps.storage)?)?)
        }
        QueryMsg::ConfigPartyA {} => Ok(to_json_binary(
            &COVENANT_CONFIG.load(deps.storage)?.party_a,
        )?),
        QueryMsg::ConfigPartyB {} => Ok(to_json_binary(
            &COVENANT_CONFIG.load(deps.storage)?.party_b,
        )?),
        QueryMsg::DepositDeadline {} => Ok(to_json_binary(&DEPOSIT_DEADLINE.load(deps.storage)?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&COVENANT_CONFIG.load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
        QueryMsg::DenomSplits {} => Ok(to_json_binary(&DENOM_SPLITS.load(deps.storage)?)?),
        QueryMsg::EmergencyCommittee {} => Ok(to_json_binary(
            &EMERGENCY_COMMITTEE_ADDR.may_load(deps.storage)?,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            next_contract,
            lockup_config,
            deposit_deadline,
            ragequit_config,
            covenant_config,
            denom_splits,
            fallback_split,
            emergency_committee,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(addr) = clock_addr {
                let clock_address = deps.api.addr_validate(&addr)?;
                CLOCK_ADDRESS.save(deps.storage, &clock_address)?;
                resp = resp.add_attribute("clock_addr", addr);
            }

            if let Some(addr) = next_contract {
                let next_contract_addr = deps.api.addr_validate(&addr)?;
                LIQUID_POOLER_ADDRESS.save(deps.storage, &next_contract_addr)?;
                resp = resp.add_attribute("next_contract", addr);
            }

            if let Some(expiry_config) = lockup_config {
                if expiry_config.is_expired(&env.block) {
                    return Err(StdError::generic_err("lockup config is already past"));
                }
                LOCKUP_CONFIG.save(deps.storage, &expiry_config)?;
                resp = resp.add_attribute("lockup_config", expiry_config.to_string());
            }

            if let Some(expiry_config) = deposit_deadline {
                if expiry_config.is_expired(&env.block) {
                    return Err(StdError::generic_err("deposit deadline is already past"));
                }
                DEPOSIT_DEADLINE.save(deps.storage, &expiry_config)?;
                resp = resp.add_attribute("deposit_deadline", expiry_config.to_string());
            }

            if let Some(addr) = emergency_committee {
                let committee_addr = deps.api.addr_validate(&addr)?;
                EMERGENCY_COMMITTEE_ADDR.save(deps.storage, &committee_addr)?;
                resp = resp.add_attribute("emergency_committee_addr", committee_addr);
            }

            if let Some(config) = *ragequit_config {
                RAGEQUIT_CONFIG.save(deps.storage, &config)?;
                resp = resp.add_attributes(config.get_response_attributes());
            }

            if let Some(config) = *covenant_config {
                COVENANT_CONFIG.save(deps.storage, &config)?;
                resp = resp.add_attribute("covenant_config", format!("{:?}", config));
            }

            if let Some(splits) = denom_splits {
                for config in splits.values() {
                    config.validate_shares_and_receiver_addresses(deps.api)?;
                }
                resp = resp.add_attribute("explicit_splits", format!("{:?}", splits));
                DENOM_SPLITS.update(deps.storage, |mut current_splits| -> StdResult<_> {
                    current_splits.explicit_splits = splits;
                    Ok(current_splits)
                })?;
            }

            if let Some(split) = fallback_split {
                split.validate_shares_and_receiver_addresses(deps.api)?;
                resp = resp.add_attribute("fallback_split", format!("{:?}", split));
                DENOM_SPLITS.update(deps.storage, |mut current_splits| -> StdResult<_> {
                    current_splits.fallback_split = Some(split);
                    Ok(current_splits)
                })?;
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
