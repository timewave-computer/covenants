
use cw2::set_contract_version;
use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response, Deps, StdResult, Binary, to_binary, Addr};

use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg};
use crate::error::ContractError;
use crate::state::{STRIDE_ATOM_RECEIVER, CLOCK_ADDRESS, NATIVE_ATOM_RECEIVER};

const CONTRACT_NAME: &str = "crates.io:stride-depositor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // can we do better with validation here?
    deps.api.addr_validate(&msg.st_atom_receiver.address)?;
    deps.api.addr_validate(&msg.atom_receiver.address)?;

    // TODO: consider re-enabling
    // let clock_contract = deps.querier.query_wasm_contract_info(msg.clock_address.to_string())?;
    // // clock should already exist, and be instantiated by the same covenant contract
    // if Addr::unchecked(clock_contract.creator) != info.sender {
    //     return Err(ContractError::InstantiatorMissmatch {})
    // }

    // avoid zero deposit configurations
    if msg.st_atom_receiver.amount.is_zero() || msg.atom_receiver.amount.is_zero() {
        return Err(ContractError::ZeroDeposit {})
    }

    // store the denominations and amounts
    STRIDE_ATOM_RECEIVER.save(deps.storage, &msg.st_atom_receiver)?;
    NATIVE_ATOM_RECEIVER.save(deps.storage, &msg.atom_receiver)?;

    // store the clock address that will be authorized to tick
    CLOCK_ADDRESS.save(deps.storage, &msg.clock_address)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => todo!(),
        ExecuteMsg::Received {} => todo!(),
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::StAtomReceiver {} => to_binary(&STRIDE_ATOM_RECEIVER.may_load(deps.storage)?),
        QueryMsg::AtomReceiver {} => to_binary(&NATIVE_ATOM_RECEIVER.may_load(deps.storage)?),
        QueryMsg::ClockAddress {} => to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?),
    }
}