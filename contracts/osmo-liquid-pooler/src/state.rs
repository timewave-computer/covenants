use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin, Uint64};
use cw_storage_plus::Item;
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

use crate::msg::{ContractState, PartyChainInfo, PartyDenomInfo, ProvidedLiquidityInfo, LiquidPoolerDenomConfig};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the incoming ticks sender
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// holder module address to verify withdrawal requests
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");

pub const NOTE_ADDRESS: Item<Addr> = Item::new("note_address");
pub const PROXY_ADDRESS: Item<String> = Item::new("proxy_address");

/// keeps track of both token amounts we provided to the pool
pub const PROVIDED_LIQUIDITY_INFO: Item<ProvidedLiquidityInfo> =
    Item::new("provided_liquidity_info");

pub const CALLBACKS: Item<Vec<String>> = Item::new("callbacks");

pub const LATEST_OSMO_POOL_SNAPSHOT: Item<Option<Pool>> = Item::new("osmo_pool");
// maps denom to balance
// pub const LATEST_PROXY_BALANCES: Item<HashMap<String, Coin>> = Item::new("proxy_balances");

pub const DENOM_CONFIG: Item<LiquidPoolerDenomConfig> = Item::new("denom_config");


pub const POOL_ID: Item<Uint64> = Item::new("pool_id");
pub const OSMOSIS_IBC_TIMEOUT: Item<Uint64> = Item::new("ibc_timeout");
pub const PARTY_1_CHAIN_INFO: Item<PartyChainInfo> = Item::new("party_1_chain_info");
pub const PARTY_2_CHAIN_INFO: Item<PartyChainInfo> = Item::new("party_2_chain_info");
pub const OSMO_TO_NEUTRON_CHANNEL_ID: Item<String> = Item::new("osmo_to_neutron_channel_id");

// pub const PARTY_1_DENOM_INFO: Item<PartyDenomInfo> = Item::new("party_1_denom_info");
// pub const PARTY_2_DENOM_INFO: Item<PartyDenomInfo> = Item::new("party_2_denom_info");

pub const OSMO_OUTPOST: Item<String> = Item::new("osmo_outpost");

pub const PROXY_COIN_1_BALANCE: Item<Coin> = Item::new("proxy_coin_1_bal");
pub const PROXY_COIN_2_BALANCE: Item<Coin> = Item::new("proxy_coin_2_bal");
