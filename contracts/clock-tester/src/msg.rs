use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint64;

#[cw_serde]
pub enum Mode {
    Accept,
    Error,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub mode: Mode,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives a tick and processes it according to the current
    /// mode.
    Tick {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the number of times the clock has received a tick and not
    /// errored in response.
    #[returns(Uint64)]
    TickCount {},
}
