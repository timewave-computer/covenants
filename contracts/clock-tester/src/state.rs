use crate::msg::Mode;
use cw_storage_plus::Item;

pub const MODE: Item<Mode> = Item::new("mode");
pub const TICK_COUNT: Item<u64> = Item::new("tick_count");
