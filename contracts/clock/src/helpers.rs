use crate::{
    error::ContractError,
    msg::ExecuteMsg::{Dequeue, Enqueue},
};
use cosmwasm_std::{to_json_binary, Addr, StdResult, WasmMsg};
use neutron_sdk::NeutronError;

pub fn enqueue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&Enqueue {})?,
        funds: vec![],
    })
}

pub fn dequeue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&Dequeue {})?,
        funds: vec![],
    })
}

pub fn verify_clock(caller: &Addr, clock_addr: &Addr) -> Result<(), NeutronError> {
    if caller != clock_addr {
        return Err(ContractError::NotClock.into());
    }

    Ok(())
}
