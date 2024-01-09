use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128, Uint64};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
use polytone::callbacks::CallbackMessage;

#[cw_serde]
pub struct InstantiateMsg {
    pub clock_address: String,
    pub holder_address: String,
    pub note_address: String,
    pub pool_id: Uint64,
    pub osmo_ibc_timeout: Uint64,
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub osmo_outpost: String,
}

#[cw_serde]
pub struct LiquidPoolerDenomConfig {
    pub latest_balances: HashMap<String, Coin>,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
}

impl LiquidPoolerDenomConfig {
    pub fn get_party_1_denom_balance(&self) -> Option<&Coin> {
        self.latest_balances
            .get(&self.party_1_denom_info.osmosis_coin.denom)
        // match self.latest_balances.get(&self.party_1_denom_info.osmosis_coin.denom) {
        //     Some(coin) => Ok(coin.clone()),
        //     None => Err(StdError::not_found("denom entry not found")),
        // }
    }

    pub fn get_party_2_denom_balance(&self) -> Option<&Coin> {
        // match self.latest_balances.get(&self.party_2_denom_info.osmosis_coin.denom) {
        //     Some(coin) => Ok(coin.clone()),
        //     None => Err(StdError::not_found("denom entry not found")),
        // }
        self.latest_balances
            .get(&self.party_2_denom_info.osmosis_coin.denom)
    }
}

#[cw_serde]
pub struct PartyDenomInfo {
    /// coin as denominated on osmosis
    pub osmosis_coin: Coin,
    /// ibc denom on liquid pooler chain
    pub neutron_denom: String,
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
    // pub party_chain_receiver_address: String,
    // pub party_chain_to_osmo_port: String,
    // pub party_chain_to_osmo_channel: String,
    pub pfm: Option<ForwardMetadata>,
    pub ibc_timeout: Uint64,
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
