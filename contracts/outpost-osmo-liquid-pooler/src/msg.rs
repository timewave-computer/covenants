use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint64;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ProvideLiquidity { pool_id: Uint64 },
}

#[cw_serde]
pub enum QueryMsg {}
