use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// address authorized to withdraw liquidity and the underlying assets
pub const WITHDRAWER: Item<Addr> = Item::new("withdrawer");
/// address of the pool we expect to withdraw assets from
pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");
