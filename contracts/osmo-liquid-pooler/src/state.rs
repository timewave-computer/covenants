use cosmwasm_std::{Addr, Coin, Binary};
use cw_storage_plus::Item;

use crate::msg::{ContractState, ProvidedLiquidityInfo};

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

pub const COIN_1: Item<Coin> = Item::new("coin_1");
pub const COIN_2: Item<Coin> = Item::new("coin_2");


pub const CALLBACKS: Item<Vec<String>> = Item::new("callbacks");

pub const LATEST_OSMO_POOL_RESPONSE: Item<Binary> = Item::new("osmo_pool");
