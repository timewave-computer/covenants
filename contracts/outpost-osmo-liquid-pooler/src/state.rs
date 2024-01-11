use cw_storage_plus::Item;
use crate::msg::JoinPoolMsgContext;

pub const PENDING_REPLY: Item<JoinPoolMsgContext> = Item::new("pending_reply");



