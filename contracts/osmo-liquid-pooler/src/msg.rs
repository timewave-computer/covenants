use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Coin, Uint64};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
use polytone::callbacks::CallbackMessage;

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub clock_address: String,
    pub holder_address: String,
    pub note_address: String,
    pub coin_1: Coin,
    pub coin_2: Coin,
    pub pool_id: Uint64,
    pub ibc_timeout: Uint64,
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub coin_1_native_denom: String,
    pub coin_2_native_denom: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    // polytone callback listener
    Callback(CallbackMessage),
}

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
    #[returns(Vec<String>)]
    Callbacks {},
    #[returns(Option<Pool>)]
    LatestPoolState {},
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
    Complete,
}

#[cw_serde]
pub struct OsmoGammPoolQueryResponse {
    pub pool: osmosis_std::types::osmosis::gamm::v1beta1::Pool,
}

#[cw_serde]
pub struct PartyChainInfo {
    pub neutron_to_party_chain_port: String,
    pub neutron_to_party_chain_channel: String,
    pub party_chain_receiver_address: String,
    pub party_chain_to_osmo_port: String,
    pub party_chain_to_osmo_channel: String,
}

// https://github.com/strangelove-ventures/packet-forward-middleware/blob/main/router/types/forward.go
#[cw_serde]
pub struct PacketMetadata {
    pub forward: Option<ForwardMetadata>,
}

#[cw_serde]
pub struct ForwardMetadata {
    pub receiver: String,
    pub port: String,
    pub channel: String,
    // pub timeout: Option<String>,
    // pub retries: Option<u8>,
}
