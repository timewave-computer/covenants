use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128
};
use covenant_utils::{
    clock::dequeue_msg,
    op_mode::{verify_caller, ContractOperationMode},
    CovenantTerms,
};

use crate::{
    error::ContractError,
    msg::{ContractState, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        CONTRACT_OP_MODE, CONTRACT_STATE, COVENANT_TERMS, LOCKUP_CONFIG, NEXT_CONTRACT,
        PARTIES_CONFIG, REFUND_CONFIG,
    },
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

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
    let op_mode = ContractOperationMode::try_init(deps.api, msg.op_mode_cfg.clone())?;

    msg.parties_config.validate_party_addresses(deps.api)?;
    if msg.lockup_config.is_expired(&env.block) {
        return Err(ContractError::Std(StdError::generic_err(
            "past lockup config",
        )));
    }
    deps.api
        .addr_validate(&msg.refund_config.party_a_refund_address)?;
    deps.api
        .addr_validate(&msg.refund_config.party_b_refund_address)?;

    NEXT_CONTRACT.save(deps.storage, &next_contract)?;
    CONTRACT_OP_MODE.save(deps.storage, &op_mode)?;
    LOCKUP_CONFIG.save(deps.storage, &msg.lockup_config)?;
    PARTIES_CONFIG.save(deps.storage, &msg.parties_config)?;
    COVENANT_TERMS.save(deps.storage, &msg.covenant_terms)?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    REFUND_CONFIG.save(deps.storage, &msg.refund_config)?;

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
    verify_caller(&info.sender, &CONTRACT_OP_MODE.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_forward(deps, env),
        ContractState::Expired => try_refund(deps, env),
        ContractState::Complete => Ok(Response::default()
            .add_attribute("contract_state", "complete")
            .add_attribute("method", "try_tick")),
    }
}

/// attempts to route any available covenant party contribution denoms to
/// the parties that were responsible for contributing that denom.
fn try_refund(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let parties = PARTIES_CONFIG.load(deps.storage)?;
    let refund_config = REFUND_CONFIG.load(deps.storage)?;

    // query holder balances
    let party_a_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        parties.party_a.native_denom,
    )?;
    let party_b_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        parties.party_b.native_denom,
    )?;

    let refund_messages: Vec<CosmosMsg> =
        match (party_a_bal.amount.is_zero(), party_b_bal.amount.is_zero()) {
            // both balances empty, nothing to refund
            (true, true) => vec![],
            // party A failed to deposit. refund party B
            (true, false) => vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: refund_config.party_b_refund_address,
                amount: vec![party_b_bal],
            })],
            // party B failed to deposit. refund party A
            (false, true) => vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: refund_config.party_a_refund_address,
                amount: vec![party_a_bal],
            })],
            // not enough balances to perform the covenant swap.
            // refund denoms to both parties.
            (false, false) => vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: refund_config.party_a_refund_address,
                    amount: vec![party_a_bal],
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: refund_config.party_b_refund_address,
                    amount: vec![party_b_bal],
                }),
            ],
        };

    Ok(Response::default()
        .add_attribute("contract_state", "expired")
        .add_attribute("method", "try_refund")
        .add_messages(refund_messages))
}

fn try_forward(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
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
        &covenant_utils::neutron::QueryMsg::DepositAddress {},
    )?;

    // if query returns None, then we error and wait
    let Some(deposit_address) = deposit_address_query else {
        return Err(ContractError::Std(StdError::not_found(
            "Next contract is not ready for receiving the funds yet",
        )));
    };

    // Transition contract state to complete
    ContractState::complete(deps.branch())?;

    let bank_msg = BankMsg::Send {
        to_address: deposit_address,
        amount,
    };

    let mut submsgs: Vec<SubMsg> = vec![];
    let _ = CONTRACT_OP_MODE.load(deps.storage).map(|op_mode| {
        match op_mode {
            ContractOperationMode::Permissioned(privileged_accounts) => {
                // given that we successfully forward the expected funds,
                // we can now dequeue from the clock and complete
                for addr in privileged_accounts.to_vec() {
                    if deps.querier.query_wasm_contract_info(addr.as_str()).is_ok() {
                        let dequeue_msg = dequeue_msg(addr.as_str()).unwrap();
                        submsgs.push(SubMsg::reply_on_error(dequeue_msg, u64::MAX));
                    }
                }
            }
            ContractOperationMode::Permissionless => {}
        }
    });

    Ok(Response::default()
        .add_message(bank_msg)
        .add_submessages(submsgs)
    )
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
        QueryMsg::OperationMode {} => {
            Ok(to_json_binary(&CONTRACT_OP_MODE.may_load(deps.storage)?)?)
        }
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        // the deposit address for swap-holder is the contract itself
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&Some(env.contract.address))?),
        QueryMsg::RefundConfig {} => Ok(to_json_binary(&REFUND_CONFIG.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::UpdateConfig {
            op_mode,
            next_contract,
            lockup_config,
            parties_config,
            covenant_terms,
            refund_config,
        } => {
            let mut resp = Response::default().add_attribute("method", "update_config");

            if let Some(op_mode_cfg) = op_mode {
                let updated_op_mode = ContractOperationMode::try_init(deps.api, op_mode_cfg)
                    .map_err(|err| StdError::generic_err(err.to_string()))?;

                CONTRACT_OP_MODE.save(deps.storage, &updated_op_mode)?;
                resp = resp.add_attribute("op_mode", format!("{:?}", updated_op_mode));
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

            if let Some(parties_config) = *parties_config {
                PARTIES_CONFIG.save(deps.storage, &parties_config)?;
                resp = resp.add_attribute("parties_config", format!("{parties_config:?}"));
            }

            if let Some(covenant_terms) = covenant_terms {
                COVENANT_TERMS.save(deps.storage, &covenant_terms)?;
                resp = resp.add_attribute("covenant_terms", format!("{covenant_terms:?}"));
            }

            if let Some(config) = refund_config {
                deps.api.addr_validate(&config.party_a_refund_address)?;
                deps.api.addr_validate(&config.party_b_refund_address)?;
                REFUND_CONFIG.save(deps.storage, &config)?;
                resp = resp.add_attribute("refund_config", format!("{config:?}"));
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // if we get a reply with id u64::MAX, we can assume it is a dequeue message
    if msg.id == u64::MAX {
        // Do nothing, whether it fails or not (dequeue messages are "fire & forget" style messages)
        Ok(Response::default())
    } else {
        Err(ContractError::UnexpectedReplyId {})
    }
}