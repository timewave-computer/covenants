use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Addr};

#[cw_serde]
pub struct InstantiateMsg {
    pub st_atom_receiver: WeightedReceiver,
    pub atom_receiver: WeightedReceiver,
    pub clock_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Tick {},
    Received {},
}

#[cw_serde]
pub enum QueryMsg {
    StAtomReceiver {},
    AtomReceiver {},
    ClockAddress {},
}

#[cw_serde]
pub struct WeightedReceiver {
        pub amount: u128,
        pub address: String,
}

