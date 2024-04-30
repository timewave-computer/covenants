use cosmwasm_std::{Addr, Uint64};
use cw_fifo::FIFOQueue;
use cw_storage_plus::Item;

pub(crate) const QUEUE: FIFOQueue<Addr> = FIFOQueue::new("front", "back", "count");
pub(crate) const PAUSED: Item<bool> = Item::new("paused");
pub(crate) const TICK_MAX_GAS: Item<Uint64> = Item::new("tmg");
pub(crate) const WHITELIST: Item<Vec<Addr>> = Item::new("whitelist");
