
use covenant_ls::msg::InstantiateMsg as LsInstantiateMsg;
use covenant_depositor::msg::InstantiateMsg as DepositorInstantiateMsg;
use covenant_lp::msg::InstantiateMsg as LpInstantiateMsg;
use covenant_clock::msg::InstantiateMsg as ClockInstantiateMsg;
use cw_storage_plus::Item;


pub const LS_INSTANTIATION_DATA: Item<(u64, LsInstantiateMsg)> = Item::new("ls_instantiation_data");
pub const LP_INSTANTIATION_DATA: Item<(u64, LpInstantiateMsg)> = Item::new("lp_instantiation_data");
pub const DEPOSITOR_INSTANTIATION_DATA: Item<(u64, DepositorInstantiateMsg)> = Item::new("depositor_instantiation_data");
pub const CLOCK_INSTANTIATION_DATA: Item<(u64, ClockInstantiateMsg)> = Item::new("clock_instantiation_data");


