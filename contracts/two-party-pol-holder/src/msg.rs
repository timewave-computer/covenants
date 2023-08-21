use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Decimal, Uint64};
use covenant_macros::{clocked, covenant_deposit_address, covenant_clock_address, covenant_next_contract};


#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// block height of covenant expiration. Position is exited 
    /// automatically upon reaching that height.
    pub expiration_height: u64,
    /// address of the next contract to forward the funds to (splitter).
    pub next_contract: Addr,
    /// optional ragequit penalty denominated in decimals
    pub ragequit_penalty: Option<Decimal>,
    /// parties engaged in the POL.
    pub whitelist_parties: Vec<Party>,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    /// initiate the ragequit
    Ragequit {},
    /// withdraw the liquidity party is entitled to
    Claim {},
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    /// one of the parties have initiated ragequit.
    /// party with an active position is free to exit at any time.
    Ragequit,
    /// covenant has reached its expiration date.
    ExpirationReached,
    /// underlying funds have been withdrawn.
    Complete,
}

#[covenant_deposit_address]
#[covenant_clock_address]
#[covenant_next_contract]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Option<Decimal>)]
    RagequitPenalty {},
    #[returns(Uint64)]
    ExpirationHeight {},
    #[returns(Vec<Party>)]
    WhitelistParties {},
}

#[cw_serde]
pub struct Party {
    pub addr: Addr,
    pub share: Uint128,
    pub provided_denom: String,
}
