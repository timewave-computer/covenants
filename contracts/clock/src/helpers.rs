use crate::{msg::ExecuteMsg::{Dequeue, Enqueue}, error::ContractError};
use cosmwasm_std::{to_binary, StdResult, WasmMsg, Addr};
use neutron_sdk::NeutronError;

pub fn enqueue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_binary(&Enqueue {})?,
        funds: vec![],
    })
}

pub fn dequeue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_binary(&Dequeue {})?,
        funds: vec![],
    })
}

pub fn verify_clock(caller: Addr, clock_addr: Addr) -> Result<(), ContractError>{
  if caller != clock_addr {
    return Err(ContractError::NotClock)
  }

  Ok(())
}
