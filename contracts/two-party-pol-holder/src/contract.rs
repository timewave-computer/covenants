use std::collections::BTreeMap;

use astroport::{asset::Asset, pair::Cw20HookMsg};
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use covenant_utils::{query_astro_pool_token, AstroportPoolTokenResponse, SplitConfig, SplitType};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::msg::CovenantType;
use crate::{
    error::ContractError,
    msg::{
        ContractState, DenomSplits, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
        RagequitConfig, RagequitState, RagequitTerms, TwoPartyPolCovenantConfig,
        TwoPartyPolCovenantParty,
    },
    state::{
        CLOCK_ADDRESS, CONTRACT_STATE, COVENANT_CONFIG, DENOM_SPLITS, DEPOSIT_DEADLINE,
        LOCKUP_CONFIG, NEXT_CONTRACT, POOL_ADDRESS, RAGEQUIT_CONFIG,
    },
};

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

    let pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;

    if msg.deposit_deadline.is_expired(&env.block) {
        return Err(ContractError::DepositDeadlineValidationError {});
    }
    if msg.lockup_config.is_expired(&env.block) {
        return Err(ContractError::LockupValidationError {});
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
        .filter_map(|(denom, split)| match split {
            SplitType::Custom(split_config) => {
                split_config
                    .validate(
                        &msg.covenant_config.party_a.router,
                        &msg.covenant_config.party_b.router,
                    )
                    .ok()?;
                Some((denom.to_string(), split_config.to_owned()))
            }
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
    POOL_ADDRESS.save(deps.storage, &pool_addr)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    RAGEQUIT_CONFIG.save(deps.storage, &msg.ragequit_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    COVENANT_CONFIG.save(deps.storage, &msg.covenant_config)?;
    DEPOSIT_DEADLINE.save(deps.storage, &msg.deposit_deadline)?;

    Ok(Response::default()
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
    match msg {
        ExecuteMsg::Ragequit {} => try_ragequit(deps, env, info),
        ExecuteMsg::Claim {} => try_claim(deps, env, info),
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::DistributeFallbackSplit { denoms } => {
            try_distribute_fallback_split(deps, env, denoms)
        }
    }
}

fn try_distribute_fallback_split(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut available_balances = Vec::new();
    let denom_splits = DENOM_SPLITS.load(deps.storage)?;

    for denom in denoms {
        if denom_splits.explicit_splits.contains_key(&denom) {
            return Err(ContractError::UnauthorizedDenomDistribution {});
        }
        let queried_coin = deps
            .querier
            .query_balance(env.contract.address.to_string(), denom)?;
        available_balances.push(queried_coin);
    }

    let fallback_distribution_messages =
        denom_splits.get_fallback_distribution_messages(available_balances);

    Ok(Response::default()
        .add_attribute("method", "try_distribute_fallback_split")
        .add_messages(fallback_distribution_messages))
}

fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let (claim_party, counterparty) = covenant_config.authorize_sender(info.sender.to_string())?;
    let pool = POOL_ADDRESS.load(deps.storage)?;
    let contract_state = CONTRACT_STATE.load(deps.storage)?;

    // if both parties already claimed everything we complete early
    if claim_party.allocation.is_zero() && counterparty.allocation.is_zero() {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
        return Ok(Response::default()
            .add_attribute("method", "try_claim")
            .add_attribute("contract_state", "complete"));
    }

    // we exit early if contract is not in ragequit or expired state
    contract_state.validate_claim_state()?;

    // find the liquidity token balance
    let lp_token_info = query_astro_pool_token(
        deps.querier,
        pool.to_string(),
        env.contract.address.to_string(),
    )?;
    // if no lp tokens are available, no point to ragequit
    if lp_token_info.balance_response.balance.is_zero() {
        return Err(ContractError::NoLpTokensAvailable {});
    }

    match covenant_config.covenant_type {
        CovenantType::Share => try_claim_share_based(
            deps,
            claim_party,
            counterparty,
            lp_token_info.balance_response.balance,
            lp_token_info.pair_info.liquidity_token.to_string(),
            pool.to_string(),
            covenant_config,
        ),
        CovenantType::Side => try_claim_side_based(
            deps,
            claim_party,
            counterparty,
            lp_token_info.balance_response.balance,
            lp_token_info.pair_info.liquidity_token.to_string(),
            pool.to_string(),
            covenant_config,
        ),
    }
}

fn try_claim_share_based(
    deps: DepsMut,
    mut claim_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    lp_token_bal: Uint128,
    lp_token_addr: String,
    pool: String,
    mut covenant_config: TwoPartyPolCovenantConfig,
) -> Result<Response, ContractError> {
    // we figure out the amounts of underlying tokens that claiming party could receive
    let claim_party_lp_token_amount = lp_token_bal
        .checked_mul_floor(claim_party.allocation)
        .map_err(|_| ContractError::FractionMulError {})?;
    let claim_party_entitled_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        pool.to_string(),
        &astroport::pair::QueryMsg::Share {
            amount: claim_party_lp_token_amount,
        },
    )?;
    // convert astro assets to coins
    let mut withdraw_coins: Vec<Coin> = vec![];
    for asset in claim_party_entitled_assets {
        withdraw_coins.push(asset.to_coin()?);
    }

    // generate the withdraw_liquidity hook for the claim party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool,
        amount: claim_party_lp_token_amount,
        msg: to_json_binary(withdraw_liquidity_hook)?,
    };

    let denom_splits = DENOM_SPLITS.load(deps.storage)?;
    let distribution_messages = denom_splits
        .get_single_receiver_distribution_messages(withdraw_coins, claim_party.router.to_string());

    // messages will contain the withdraw liquidity message followed
    // by transfer of underlying assets to the corresponding router
    let mut messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token_addr,
        msg: to_json_binary(withdraw_msg)?,
        funds: vec![],
    })];

    // Append distribution messages
    messages.extend(distribution_messages);

    claim_party.allocation = Decimal::zero();

    // if other party had not claimed yet, we assign it the full position
    if !counterparty.allocation.is_zero() {
        counterparty.allocation = Decimal::one();
    } else {
        // otherwise both parties claimed everything and we can complete
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
    }

    covenant_config.update_parties(claim_party, counterparty);

    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;

    Ok(Response::default()
        .add_attribute("method", "claim_share_based")
        .add_messages(messages))
}

fn try_claim_side_based(
    deps: DepsMut,
    mut claim_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    lp_token_bal: Uint128,
    lp_token_addr: String,
    pool: String,
    mut covenant_config: TwoPartyPolCovenantConfig,
) -> Result<Response, ContractError> {
    // we figure out the amount of tokens to be expected
    let entitled_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        pool.to_string(),
        &astroport::pair::QueryMsg::Share {
            amount: lp_token_bal,
        },
    )?;
    // convert astro assets to coins
    let mut withdraw_coins: Vec<Coin> = vec![];
    for asset in entitled_assets {
        withdraw_coins.push(asset.to_coin()?);
    }

    // generate the withdraw_liquidity hook for the claim party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool,
        amount: lp_token_bal,
        msg: to_json_binary(withdraw_liquidity_hook)?,
    };

    let denom_splits = DENOM_SPLITS.load(deps.storage)?;
    let distribution_messages: Vec<CosmosMsg> =
        denom_splits.get_shared_distribution_messages(withdraw_coins);

    // messages will contain the withdraw liquidity message followed
    // by transfer of underlying assets to the corresponding router
    let mut messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token_addr,
        msg: to_json_binary(withdraw_msg)?,
        funds: vec![],
    })];

    // Append distribution messages
    messages.extend(distribution_messages);

    claim_party.allocation = Decimal::zero();
    counterparty.allocation = Decimal::zero();
    covenant_config.update_parties(claim_party, counterparty);

    // update the states
    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;

    Ok(Response::default()
        .add_attribute("method", "claim_side_based")
        .add_messages(messages))
}

fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state = CONTRACT_STATE.load(deps.storage)?;
    match state {
        ContractState::Instantiated => try_deposit(deps, env, info),
        ContractState::Active => check_expiration(deps, env),
        ContractState::Complete => Ok(Response::default()
            .add_attribute("method", "tick")
            .add_attribute("contract_state", state.to_string())),
        ContractState::Expired | ContractState::Ragequit => {
            let pool = POOL_ADDRESS.load(deps.storage)?;
            let lp_token_bal = query_astro_pool_token(
                deps.querier,
                pool.to_string(),
                env.contract.address.to_string(),
            )?
            .balance_response
            .balance;
            let state = if lp_token_bal.is_zero() {
                CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
                ContractState::Complete
            } else {
                state
            };
            Ok(Response::default()
                .add_attribute("method", "tick")
                .add_attribute("lp_token_bal", lp_token_bal)
                .add_attribute("contract_state", state.to_string()))
        }
    }
}

fn try_deposit(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let config = COVENANT_CONFIG.load(deps.storage)?;
    let deposit_deadline = DEPOSIT_DEADLINE.load(deps.storage)?;

    // assert the balances
    let party_a_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        config.party_a.contribution.denom,
    )?;
    let party_b_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        config.party_b.contribution.denom,
    )?;

    let party_a_fulfilled = config.party_a.contribution.amount <= party_a_bal.amount;
    let party_b_fulfilled = config.party_b.contribution.amount <= party_b_bal.amount;

    // note: even if both parties deposit their funds in time,
    // it is important to trigger this method before the expiry block
    // if deposit deadline is due we complete and refund
    if deposit_deadline.is_expired(&env.block) {
        let refund_messages: Vec<CosmosMsg> =
            match (party_a_bal.amount.is_zero(), party_b_bal.amount.is_zero()) {
                // both balances empty, we complete
                (true, true) => {
                    CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
                    return Ok(Response::default()
                        .add_attribute("method", "try_deposit")
                        .add_attribute("state", "complete"));
                }
                // refund party B
                (true, false) => vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.party_b.router,
                    amount: vec![party_b_bal],
                })],
                // refund party A
                (false, true) => vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.party_a.router,
                    amount: vec![party_a_bal],
                })],
                // refund both
                (false, false) => vec![
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.party_a.router.to_string(),
                        amount: vec![party_a_bal],
                    }),
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.party_b.router,
                        amount: vec![party_b_bal],
                    }),
                ],
            };
        return Ok(Response::default()
            .add_attribute("method", "try_deposit")
            .add_attribute("action", "refund")
            .add_messages(refund_messages));
    }

    if !party_a_fulfilled || !party_b_fulfilled {
        // if deposit deadline is not yet due and both parties did not fulfill we error
        return Err(ContractError::InsufficientDeposits {});
    }

    // LiquidPooler is the next contract
    let liquid_pooler = NEXT_CONTRACT.load(deps.storage)?;
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

fn check_expiration(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
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

fn try_ragequit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // first we error out if ragequit is disabled
    let rq_config = match RAGEQUIT_CONFIG.load(deps.storage)? {
        RagequitConfig::Disabled => return Err(ContractError::RagequitDisabled {}),
        RagequitConfig::Enabled(terms) => terms,
    };
    let current_state = CONTRACT_STATE.load(deps.storage)?;
    let lockup_config = LOCKUP_CONFIG.load(deps.storage)?;
    let covenant_config = COVENANT_CONFIG.load(deps.storage)?;
    let pool = POOL_ADDRESS.load(deps.storage)?;

    // ragequit is only possible when contract is in Active state.
    if current_state != ContractState::Active {
        return Err(ContractError::NotActive {});
    }
    // we also validate an edge case where it did expire but
    // did not receive a tick yet. tick is then required to advance.
    if lockup_config.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // we query our own liquidity token balance and address
    let lp_token_info = query_astro_pool_token(
        deps.querier,
        pool.to_string(),
        env.contract.address.to_string(),
    )?;

    // if no lp tokens are available, no point to ragequit
    if lp_token_info.balance_response.balance.is_zero() {
        return Err(ContractError::NoLpTokensAvailable {});
    }

    // authorize the message sender
    let (rq_party, counterparty) = covenant_config.authorize_sender(info.sender.to_string())?;

    // depending on the type of ragequit configuration,
    // different logic we execute
    match covenant_config.covenant_type {
        CovenantType::Share => try_handle_share_based_ragequit(
            deps,
            rq_party,
            counterparty,
            pool,
            lp_token_info,
            rq_config,
            covenant_config,
        ),
        CovenantType::Side => try_handle_side_based_ragequit(
            deps,
            rq_party,
            counterparty,
            pool,
            lp_token_info,
            rq_config,
            covenant_config,
        ),
    }
}

/// in a side-based situation, each party owns the denom that
/// they provided.
/// in case of a ragequit, the penalty is denominated as a percentage
/// of the ragequitting party's denom. because of that, on ragequit,
/// entire covenant LP token balance is burned, in turn withdrawing
/// all underlying tokens to the holder.
/// after applying the penalty, all tokens are routed back to the
/// covenant parties. the covenant then completes.
pub fn try_handle_side_based_ragequit(
    deps: DepsMut,
    mut ragequit_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    pool: Addr,
    lp_token_response: AstroportPoolTokenResponse,
    mut rq_terms: RagequitTerms,
    mut covenant_config: TwoPartyPolCovenantConfig,
) -> Result<Response, ContractError> {
    // apply the ragequit penalty and get the new splits
    let denom_splits = DENOM_SPLITS.load(deps.storage)?;
    let updated_denom_splits =
        denom_splits.apply_penalty(rq_terms.penalty, &ragequit_party, &counterparty)?;
    DENOM_SPLITS.save(deps.storage, &updated_denom_splits)?;

    // for withdrawing the entire LP position we query full share
    let ragequit_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        pool.to_string(),
        &astroport::pair::QueryMsg::Share {
            amount: lp_token_response.balance_response.balance,
        },
    )?;

    // reflect the ragequit in ragequit config
    let rq_state = RagequitState::from_share_response(ragequit_assets, ragequit_party.clone())?;
    rq_terms.state = Some(rq_state.clone());

    // generate the withdraw_liquidity hook for the ragequitting party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool.to_string(),
        amount: lp_token_response.balance_response.balance,
        msg: to_json_binary(withdraw_liquidity_hook)?,
    };

    let balances = rq_state.coins.clone();
    let distribution_messages = updated_denom_splits.get_shared_distribution_messages(balances);

    // messages will contain the withdraw liquidity message followed
    // by transfer of underlying assets to the corresponding router
    let mut messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token_response.pair_info.liquidity_token.to_string(),
        msg: to_json_binary(withdraw_msg)?,
        funds: vec![],
    })];

    // Append distribution messages
    messages.extend(distribution_messages);

    ragequit_party.allocation = Decimal::zero();
    counterparty.allocation = Decimal::zero();
    covenant_config.update_parties(ragequit_party.clone(), counterparty);

    // update the states
    RAGEQUIT_CONFIG.save(deps.storage, &RagequitConfig::Enabled(rq_terms))?;
    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;

    Ok(Response::default()
        .add_attribute("method", "ragequit_side_based")
        .add_attribute("controller_chain_caller", ragequit_party.controller_addr)
        .add_attribute("host_chain_caller", ragequit_party.host_addr)
        .add_messages(messages))
}

/// in share-based situation, each party owns a fraction x_i of
/// the entire LP position, where Σx_i = 1.
/// in case of a ragequit, the penalty is denominated in LP tokens.
/// this happens by the ragequitting party forfeiting a penalty P
/// of the entire covenant POL position to the counterparty,
/// denominated in `cosmwasm_std::Decimal`.
/// this configuration is beneficial for POL covenants where maintaining
/// pool depth is a priority, as only x_i - P of the entire liquidity
/// is withdrawn from the pool (counterparty position remains active).
pub fn try_handle_share_based_ragequit(
    deps: DepsMut,
    mut ragequit_party: TwoPartyPolCovenantParty,
    mut counterparty: TwoPartyPolCovenantParty,
    pool: Addr,
    lp_token_response: AstroportPoolTokenResponse,
    mut rq_terms: RagequitTerms,
    mut covenant_config: TwoPartyPolCovenantConfig,
) -> Result<Response, ContractError> {
    // apply the ragequit penalty and get the new splits
    let denom_splits = DENOM_SPLITS.load(deps.storage)?;
    let updated_denom_splits =
        denom_splits.apply_penalty(rq_terms.penalty, &ragequit_party, &counterparty)?;
    DENOM_SPLITS.save(deps.storage, &updated_denom_splits)?;

    // apply the ragequit penalty
    ragequit_party.allocation -= rq_terms.penalty;

    // we figure out the amounts of underlying tokens that rq party would receive
    let rq_party_lp_token_amount = lp_token_response
        .balance_response
        .balance
        .checked_mul_floor(ragequit_party.allocation)
        .map_err(|_| ContractError::FractionMulError {})?;
    let rq_entitled_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        pool.to_string(),
        &astroport::pair::QueryMsg::Share {
            amount: rq_party_lp_token_amount,
        },
    )?;

    // reflect the ragequit in ragequit config
    let rq_state = RagequitState::from_share_response(rq_entitled_assets, ragequit_party.clone())?;
    rq_terms.state = Some(rq_state.clone());

    // generate the withdraw_liquidity hook for the ragequitting party
    let withdraw_liquidity_hook = &Cw20HookMsg::WithdrawLiquidity { assets: vec![] };
    let withdraw_msg = &Cw20ExecuteMsg::Send {
        contract: pool.to_string(),
        amount: rq_party_lp_token_amount,
        msg: to_json_binary(withdraw_liquidity_hook)?,
    };

    let balances = rq_state.coins.clone();
    let distribution_messages = updated_denom_splits
        .get_single_receiver_distribution_messages(balances, ragequit_party.router.to_string());

    // messages will contain the withdraw liquidity message followed
    // by transfer of underlying assets to the corresponding router
    let mut messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token_response.pair_info.liquidity_token.to_string(),
        msg: to_json_binary(withdraw_msg)?,
        funds: vec![],
    })];

    // Append distribution messages
    messages.extend(distribution_messages);

    // after building the messages we can finalize the config updates.
    // rq party is now entitled to nothing. counterparty owns the entire position.
    ragequit_party.allocation = Decimal::zero();
    counterparty.allocation = Decimal::one();
    covenant_config.update_parties(ragequit_party.clone(), counterparty);

    // update the states
    RAGEQUIT_CONFIG.save(deps.storage, &RagequitConfig::Enabled(rq_terms))?;
    COVENANT_CONFIG.save(deps.storage, &covenant_config)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Ragequit)?;

    Ok(Response::default()
        .add_attribute("method", "ragequit_share_based")
        .add_attribute("controller_chain_caller", ragequit_party.controller_addr)
        .add_attribute("host_chain_caller", ragequit_party.host_addr)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.load(deps.storage)?)?),
        QueryMsg::RagequitConfig {} => Ok(to_json_binary(&RAGEQUIT_CONFIG.load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_json_binary(&LOCKUP_CONFIG.load(deps.storage)?)?),
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.load(deps.storage)?)?),
        QueryMsg::NextContract {} => Ok(to_json_binary(&NEXT_CONTRACT.load(deps.storage)?)?),
        QueryMsg::PoolAddress {} => Ok(to_json_binary(&POOL_ADDRESS.load(deps.storage)?)?),
        QueryMsg::ConfigPartyA {} => Ok(to_json_binary(
            &COVENANT_CONFIG.load(deps.storage)?.party_a,
        )?),
        QueryMsg::ConfigPartyB {} => Ok(to_json_binary(
            &COVENANT_CONFIG.load(deps.storage)?.party_b,
        )?),
        QueryMsg::DepositDeadline {} => Ok(to_json_binary(&DEPOSIT_DEADLINE.load(deps.storage)?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&COVENANT_CONFIG.load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
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
            deposit_deadline,
            pool_address,
            ragequit_config,
            covenant_config,
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

            if let Some(expiry_config) = deposit_deadline {
                if expiry_config.is_expired(&env.block) {
                    return Err(StdError::generic_err("deposit deadline is already past"));
                }
                DEPOSIT_DEADLINE.save(deps.storage, &expiry_config)?;
                resp = resp.add_attribute("deposit_deadline", expiry_config.to_string());
            }

            if let Some(addr) = pool_address {
                let pool_addr = deps.api.addr_validate(&addr)?;
                POOL_ADDRESS.save(deps.storage, &pool_addr)?;
                resp = resp.add_attribute("pool_addr", pool_addr);
            }

            if let Some(config) = *ragequit_config {
                RAGEQUIT_CONFIG.save(deps.storage, &config)?;
                resp = resp.add_attributes(config.get_response_attributes());
            }

            if let Some(config) = *covenant_config {
                COVENANT_CONFIG.save(deps.storage, &config)?;
                resp = resp.add_attribute("todo", "todo");
            }

            Ok(resp)
        }
        MigrateMsg::UpdateCodeId { data: _ } => todo!(),
    }
}
