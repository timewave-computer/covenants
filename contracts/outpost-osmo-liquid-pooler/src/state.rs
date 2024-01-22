use crate::msg::CallerContext;
use cw_storage_plus::Item;

pub const PENDING_REPLY: Item<CallerContext> = Item::new("pending_reply");
