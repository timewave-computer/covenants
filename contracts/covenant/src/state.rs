use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use neutron_sdk::bindings::msg::IbcFee;

pub const LS_CODE: Item<u64> = Item::new("ls_code");
pub const LP_CODE: Item<u64> = Item::new("lp_code");
pub const DEPOSITOR_CODE: Item<u64> = Item::new("depositor_code");
pub const CLOCK_CODE: Item<u64> = Item::new("clock_code");
pub const HOLDER_CODE: Item<u64> = Item::new("holder_code");

pub const POOL_ADDRESS: Item<String> = Item::new("pool_address");
pub const IBC_TIMEOUT: Item<u64> = Item::new("ibc_timeout");
pub const IBC_FEE: Item<IbcFee> = Item::new("ibc_fee");

pub const PRESET_LS_FIELDS: Item<covenant_ls::msg::PresetLsFields> = Item::new("preset_ls_fields");
pub const PRESET_LP_FIELDS: Item<covenant_lp::msg::PresetLpFields> = Item::new("preset_lp_fields");
pub const PRESET_DEPOSITOR_FIELDS: Item<covenant_depositor::msg::PresetDepositorFields> =
    Item::new("preset_depositor_fields");
pub const PRESET_CLOCK_FIELDS: Item<covenant_clock::msg::PresetClockFields> =
    Item::new("preset_clock_fields");
pub const PRESET_HOLDER_FIELDS: Item<covenant_holder::msg::PresetHolderFields> =
    Item::new("preset_holder_fields");

// addresses
pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const COVENANT_LP_ADDR: Item<Addr> = Item::new("covenant_lp_addr");
pub const COVENANT_LS_ADDR: Item<Addr> = Item::new("covenant_ls_addr");
pub const COVENANT_DEPOSITOR_ADDR: Item<Addr> = Item::new("covenant_depositor_addr");
pub const COVENANT_HOLDER_ADDR: Item<Addr> = Item::new("covenant_holder_addr");
