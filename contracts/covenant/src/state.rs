use cw_storage_plus::Item;

pub const LS_CODE: Item<u64> = Item::new("ls_code");
pub const LP_CODE: Item<u64> = Item::new("lp_code");
pub const DEPOSITOR_CODE: Item<u64> = Item::new("depositor_code");
pub const CLOCK_CODE: Item<u64> = Item::new("clock_code");
pub const HOLDER_CODE: Item<u64> = Item::new("holder_code");

pub const LS_INSTANTIATION_DATA: Item<covenant_ls::msg::InstantiateMsg> = Item::new("ls_instantiation_data");
pub const LP_INSTANTIATION_DATA: Item<covenant_lp::msg::InstantiateMsg> = Item::new("lp_instantiation_data");
pub const DEPOSITOR_INSTANTIATION_DATA: Item<covenant_depositor::msg::InstantiateMsg> = Item::new("depositor_instantiation_data");
pub const CLOCK_INSTANTIATION_DATA: Item<covenant_clock::msg::InstantiateMsg> = Item::new("clock_instantiation_data");
pub const HOLDER_INSTANTIATION_DATA: Item<covenant_holder::msg::InstantiateMsg> = Item::new("holder_instantiation_data");

pub const IBC_MSG_TRANSFER_TIMEOUT_TIMESTAMP: Item<u64> = Item::new("timeout");

// replies
pub const COVENANT_CLOCK_ADDR: Item<String> = Item::new("covenant_clock_addr");
pub const COVENANT_LP_ADDR: Item<String> = Item::new("covenant_lp_addr");
pub const COVENANT_LS_ADDR: Item<String> = Item::new("covenant_ls_addr");
pub const COVENANT_DEPOSITOR_ADDR: Item<String> = Item::new("covenant_depositor_addr");
pub const COVENANT_HOLDER_ADDR: Item<String> = Item::new("covenant_holder_addr");
pub const COVENANT_STRIDE_ICA_ADDR: Item<String> = Item::new("covenant_stride_ica_addr");
