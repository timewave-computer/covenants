use cosmwasm_schema::cw_serde;
use covenant_macros::clocked;


#[cw_serde]
pub struct InstantiateMsg {
}


#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
}