
// use covenant_ls::msg::InstantiateMsg as LsInstantiateMsg;
// use covenant_depositor::msg::InstantiateMsg as DepositorInstantiateMsg;
// use covenant_lp::msg::InstantiateMsg as LpInstantiateMsg;
// use covenant_clock::msg::InstantiateMsg as ClockInstantiateMsg;
// use covenant_holder::msg::InstantiateMsg as HolderInstantiateMsg;

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
// pub const HOLDER_INSTANTIATION_DATA: Item<covenant_holder::msg::InstantiateMsg> = Item::new("holder_instantiation_data");

// replies
pub const COVENANT_CLOCK: Item<String> = Item::new("covenant_clock");
pub const COVENANT_LP: Item<String> = Item::new("covenant_lp");
pub const COVENANT_LS: Item<String> = Item::new("covenant_ls");
pub const COVENANT_DEPOSITOR: Item<String> = Item::new("covenant_depositor");
pub const COVENANT_HOLDER: Item<String> = Item::new("covenant_holder");