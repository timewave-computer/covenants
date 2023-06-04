use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::WeightedReceiver;

// addr and amount of atom to liquid stake on stride
pub const STRIDE_ATOM_RECEIVER: Item<WeightedReceiver> = Item::new("stride_atom_receiver");
// addr and amount of atom
pub const NATIVE_ATOM_RECEIVER: Item<WeightedReceiver> = Item::new("native_atom_receiver");
// store the clock address to verify calls
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");


