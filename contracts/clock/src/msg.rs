use cosmwasm_schema::{cw_serde, QueryResponses};

use covenant_clock_derive::clocked;

#[cw_serde]
pub struct InstantiateMsg {}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    Enqueue {},
    Dequeue {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
