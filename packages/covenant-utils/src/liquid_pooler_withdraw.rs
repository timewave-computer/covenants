use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use covenant_macros::{covenant_holder_distribute, covenant_lper_withdraw};

#[covenant_lper_withdraw]
#[covenant_holder_distribute]
#[cw_serde]
pub enum WithdrawLPMsgs {}
