use crate::msg::ExecuteMsg::{Dequeue, Enqueue};
use cosmwasm_std::{to_binary, StdResult, WasmMsg};

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
