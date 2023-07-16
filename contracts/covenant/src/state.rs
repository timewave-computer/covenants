use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const LS_CODE: Item<u64> = Item::new("ls_code");
pub const LP_CODE: Item<u64> = Item::new("lp_code");
pub const DEPOSITOR_CODE: Item<u64> = Item::new("depositor_code");
pub const CLOCK_CODE: Item<u64> = Item::new("clock_code");
pub const HOLDER_CODE: Item<u64> = Item::new("holder_code");

pub const POOL_ADDRESS: Item<String> = Item::new("pool_address");

// addresses
pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const COVENANT_LP_ADDR: Item<Addr> = Item::new("covenant_lp_addr");
pub const COVENANT_LS_ADDR: Item<Addr> = Item::new("covenant_ls_addr");
pub const COVENANT_DEPOSITOR_ADDR: Item<Addr> = Item::new("covenant_depositor_addr");
pub const COVENANT_HOLDER_ADDR: Item<Addr> = Item::new("covenant_holder_addr");
