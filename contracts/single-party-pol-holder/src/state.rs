use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_utils::Expiration;

/// address authorized to withdraw liquidity and the underlying assets
pub const WITHDRAWER: Item<Addr> = Item::new("withdrawer");
/// Addr that we withdraw the liquidity to
pub const WITHDRAW_TO: Item<Addr> = Item::new("withdraw_to");
/// address of the pool we expect to withdraw assets from
pub const POOLER_ADDRESS: Item<Addr> = Item::new("pool_address");
/// The lockup period of the LP tokens
pub const LOCKUP_PERIOD: Item<Expiration> = Item::new("lockup_period");
