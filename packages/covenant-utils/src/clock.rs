use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdError, StdResult, WasmMsg};
use neutron_sdk::NeutronError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ClockError {
    #[error("Caller is not the clock, only clock can tick contracts")]
    NotClock,
}

impl From<ClockError> for NeutronError {
    fn from(val: ClockError) -> Self {
        NeutronError::Std(StdError::generic_err(val.to_string()))
    }
}

#[cw_serde]
enum ClockMsg {
    /// Enqueues the message sender for ticks (serialized as messages
    /// in the form `{"tick": {}}`). The sender will continue to
    /// receive ticks until sending a `Dequeue {}` message. Only
    /// callable if the message sender is not currently enqueued and
    /// is a contract.
    Enqueue {},
    /// Dequeues the message sender stopping them from receiving
    /// ticks. Only callable if the message sender is currently
    /// enqueued.
    Dequeue {},
}

pub fn enqueue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&ClockMsg::Enqueue {})?,
        funds: vec![],
    })
}

pub fn dequeue_msg(addr: &str) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&ClockMsg::Dequeue {})?,
        funds: vec![],
    })
}

pub fn verify_clock(caller: &Addr, clock_addr: &Addr) -> Result<(), NeutronError> {
    if caller != clock_addr {
        return Err(ClockError::NotClock.into());
    }

    Ok(())
}
