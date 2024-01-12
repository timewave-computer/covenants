use crate::msg::JoinPoolMsgContext;
use cw_storage_plus::Item;

pub const PENDING_REPLY: Item<JoinPoolMsgContext> = Item::new("pending_reply");
