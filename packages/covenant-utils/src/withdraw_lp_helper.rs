use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Decimal, StdError, WasmMsg};
use covenant_macros::{covenant_holder_distribute, covenant_lper_withdraw};
use cw_storage_plus::Item;

/// Emergency committee address
pub const EMERGENCY_COMMITTEE_ADDR: Item<Addr> = Item::new("e_c_a");

#[covenant_lper_withdraw]
#[covenant_holder_distribute]
#[cw_serde]
pub enum WithdrawLPMsgs {}

pub fn generate_withdraw_msg(
    contract_addr: String,
    percentage: Option<Decimal>,
) -> Result<WasmMsg, StdError> {
    Ok(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&WithdrawLPMsgs::Withdraw { percentage })?,
        funds: vec![],
    })
}
