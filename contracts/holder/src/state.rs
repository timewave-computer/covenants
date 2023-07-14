use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const WITHDRAWER: Item<Addr> = Item::new("withdrawer");
pub const LP_ADDRESS: Item<String> = Item::new("lp_address");