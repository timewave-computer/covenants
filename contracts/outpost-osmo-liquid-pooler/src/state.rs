use cw_storage_plus::Item;
use crate::msg::CallerContext;

pub const PENDING_REPLY: Item<CallerContext> = Item::new("pending_reply");
