#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use covenant_utils::withdraw_lp_helper::{generate_withdraw_msg, EMERGENCY_COMMITTEE_ADDR};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{LOCKUP_PERIOD, POOLER_ADDRESS, WITHDRAWER, WITHDRAW_STATE, WITHDRAW_TO};

const CONTRACT_NAME: &str = "crates.io:covenant-holder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    deps.api.debug("WASMDEBUG: holder instantiate");
    let mut resp = Response::default().add_attribute("method", "instantiate");

    // withdrawer is optional on instantiation; can be set later
    if let Some(addr) = msg.withdrawer {
        WITHDRAWER.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
        resp = resp.add_attribute("withdrawer", addr);
    };

    if let Some(addr) = msg.withdraw_to {
        WITHDRAW_TO.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
        resp = resp.add_attribute("withdraw_to", addr);
    };
    
    if let Some(addr) = msg.emergency_committee_addr {
        EMERGENCY_COMMITTEE_ADDR.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
        resp = resp.add_attribute("emergency_committee", addr);
    };

    ensure!(
        !msg.lockup_period.is_expired(&_env.block),
        ContractError::MustBeFutureLockupPeriod {}
    );
    LOCKUP_PERIOD.save(deps.storage, &msg.lockup_period)?;
    POOLER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&msg.pooler_address)?)?;

    Ok(resp.add_attribute("pool_address", msg.pooler_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Withdrawer {} => Ok(to_json_binary(&WITHDRAWER.may_load(deps.storage)?)?),
        QueryMsg::WithdrawTo {} => Ok(to_json_binary(&WITHDRAW_TO.may_load(deps.storage)?)?),
        QueryMsg::PoolerAddress {} => Ok(to_json_binary(&POOLER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::EmergencyCommitteeAddr {} => Ok(to_json_binary(&EMERGENCY_COMMITTEE_ADDR.may_load(deps.storage)?)?),
        QueryMsg::LockupConfig {} => Ok(to_json_binary(&LOCKUP_PERIOD.load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Claim {} => try_claim(deps, env, info),
        ExecuteMsg::Distribute {} => try_distribute(deps, info),
        ExecuteMsg::WithdrawFailed {} => try_withdraw_failed(deps, info),
        ExecuteMsg::EmergencyWithdraw {} => try_emergency_withdraw(deps, info),
    }
}

fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    if WITHDRAW_STATE.load(deps.storage).is_ok() {
        return Err(ContractError::WithdrawAlreadyStarted {});
    }

    let lockup_period = LOCKUP_PERIOD.load(deps.storage)?;
    ensure!(
        lockup_period.is_expired(&env.block),
        ContractError::LockupPeriodNotOver(lockup_period.to_string())
    );

    let withdrawer = WITHDRAWER
        .load(deps.storage)
        .map_err(|_| ContractError::NoWithdrawer {})?;
    ensure!(info.sender == withdrawer, ContractError::Unauthorized {});

    WITHDRAW_TO
        .load(deps.storage)
        .map_err(|_| ContractError::NoWithdrawTo {})?;

    let pooler_address = POOLER_ADDRESS.load(deps.storage)?;

    let withdraw_msg = generate_withdraw_msg(pooler_address.to_string(), None)?;

    WITHDRAW_STATE.save(deps.storage, &true)?;

    Ok(Response::default().add_message(withdraw_msg))
}

fn try_emergency_withdraw(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Make sure we are not withdrawing already
    if WITHDRAW_STATE.load(deps.storage).is_ok() {
        return Err(ContractError::WithdrawAlreadyStarted {});
    }

    let committee_addr = EMERGENCY_COMMITTEE_ADDR.load(deps.storage)?;
    ensure!(
        info.sender == committee_addr,
        ContractError::Unauthorized {}
    );

    let pooler_address = POOLER_ADDRESS.load(deps.storage)?;
    let withdraw_msg = generate_withdraw_msg(pooler_address.to_string(), None)?;

    WITHDRAW_STATE.save(deps.storage, &true)?;

    Ok(Response::default().add_message(withdraw_msg))
}

fn try_distribute(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let pooler_addr = POOLER_ADDRESS.load(deps.storage)?;
    ensure!(info.sender == pooler_addr, ContractError::Unauthorized {});

    let withdraw_to_addr = WITHDRAW_TO
        .load(deps.storage)
        .map_err(|_| ContractError::NoWithdrawTo {})?;

    ensure!(info.funds.len() == 2, ContractError::InvalidFunds {});

    WITHDRAW_STATE.remove(deps.storage);

    let send_msg = BankMsg::Send {
        to_address: withdraw_to_addr.to_string(),
        amount: info.funds,
    };

    Ok(Response::default().add_message(send_msg))
}

/// We don't need to do much if the withdraw failed.
/// We just need to ensure the caller is the pooler, and remove the withdraw_state storage
fn try_withdraw_failed(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let pooler_addr = POOLER_ADDRESS.load(deps.storage)?;
    ensure!(info.sender == pooler_addr, ContractError::Unauthorized {});

    WITHDRAW_STATE.remove(deps.storage);

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            withdrawer,
            withdraw_to,
            pooler_address,
            lockup_period,
            emergency_committee,
        } => {
            let mut response = Response::default().add_attribute("method", "update_withdrawer");

            if let Some(addr) = withdrawer {
                WITHDRAWER.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("withdrawer", addr);
            }

            if let Some(addr) = withdraw_to {
                WITHDRAW_TO.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("withdraw_to", addr);
            }

            if let Some(addr) = emergency_committee {
                EMERGENCY_COMMITTEE_ADDR.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("emergency_committee", addr);
            }

            if let Some(addr) = pooler_address {
                POOLER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&addr)?)?;
                response = response.add_attribute("pool_address", addr);
            }

            if let Some(expires) = lockup_period {
                // validate that the new lockup period is in the future
                ensure!(
                    !expires.is_expired(&env.block),
                    ContractError::MustBeFutureLockupPeriod {}
                );
                LOCKUP_PERIOD.save(deps.storage, &expires)?;
                response = response.add_attribute("lockup_period", expires.to_string());
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}
