use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
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
    pub lp_token_denom: String,
    pub slippage_tolerance: Option<Decimal>,
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
}

#[cw_serde]
pub struct LiquidityProvisionConfig {
    pub latest_balances: HashMap<String, Coin>,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub pool_id: Uint64,
    pub outpost: String,
    pub lp_token_denom: String,
    pub slippage_tolerance: Option<Decimal>,
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
}

#[cw_serde]
pub struct IbcConfig {
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub osmo_ibc_timeout: Uint64,
}

impl LiquidityProvisionConfig {
    pub fn get_party_1_proxy_balance(&self) -> Option<&Coin> {
        self.latest_balances
            .get(&self.party_1_denom_info.osmosis_coin.denom)
    }

    pub fn get_party_2_proxy_balance(&self) -> Option<&Coin> {
        self.latest_balances
            .get(&self.party_2_denom_info.osmosis_coin.denom)
    }

    pub fn get_osmo_outpost_provide_liquidity_message(
        &self,
    ) -> covenant_outpost_osmo_liquid_pooler::msg::ExecuteMsg {
        covenant_outpost_osmo_liquid_pooler::msg::ExecuteMsg::ProvideLiquidity {
            pool_id: Uint64::new(self.pool_id.u64()),
            min_pool_asset_ratio: self.expected_spot_price - self.acceptable_price_spread,
            max_pool_asset_ratio: self.expected_spot_price + self.acceptable_price_spread,
            // if no slippage tolerance is passed, we use 0
            slippage_tolerance: self.slippage_tolerance.unwrap_or_default(),
        }
    }

    pub fn reset_latest_proxy_balances(&mut self) {
        self.latest_balances.remove(&self.party_1_denom_info.osmosis_coin.denom);
        self.latest_balances.remove(&self.party_1_denom_info.osmosis_coin.denom);
    }
}

#[cw_serde]
pub struct PartyDenomInfo {
    /// coin as denominated on osmosis
    pub osmosis_coin: Coin,
    /// ibc denom on liquid pooler chain
    pub neutron_denom: String,
}

impl PartyDenomInfo {
    pub fn get_osmo_bal(&self) -> Uint128 {
        self.osmosis_coin.amount
    }
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
    #[returns(Option<String>)]
    ProxyAddress {},
    #[returns(Vec<Coin>)]
    ProxyBalances {},
    #[returns(Vec<String>)]
    Callbacks {},
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
pub struct PartyChainInfo {
    // todo: reconsider naming here
    pub neutron_to_party_chain_port: String,
    pub neutron_to_party_chain_channel: String,
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
}
