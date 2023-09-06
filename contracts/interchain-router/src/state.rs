use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::DestinationConfig;

pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const DESTINATION_CONFIG: Item<DestinationConfig> = Item::new("destination_config");