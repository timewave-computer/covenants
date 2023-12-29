use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64, QueryRequest, Empty, CosmosMsg, Coin};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub clock_address: String,
    pub holder_address: String,
    pub note_address: String,
    pub coin_1: Coin,
    pub coin_2: Coin,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_clock_address]
#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(ProvidedLiquidityInfo)]
    ProvidedLiquidityInfo {},
    #[returns(Option<String>)]
    ProxyAddress {},
}

/// keeps track of provided asset liquidities in `Uint128`.
#[cw_serde]
pub struct ProvidedLiquidityInfo {
    pub provided_amount_a: Uint128,
    pub provided_amount_b: Uint128,
}

/// state of the LP state machine
#[cw_serde]
pub enum ContractState {
    Instantiated,
    ProxyCreated,
    ProxyFunded,
    Active,
    Complete,
}
