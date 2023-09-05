use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use neutron_sdk::bindings::msg::IbcFee;

use crate::msg::Timeouts;

/// contract code for the liquid staker contract
pub const LS_CODE: Item<u64> = Item::new("ls_code");
/// contract code for the liquid pooler contract
pub const LP_CODE: Item<u64> = Item::new("lp_code");
/// contract code for the depositor contract
pub const DEPOSITOR_CODE: Item<u64> = Item::new("depositor_code");
/// contract code for the clock contract
pub const CLOCK_CODE: Item<u64> = Item::new("clock_code");
/// contract code for the holder contract
pub const HOLDER_CODE: Item<u64> = Item::new("holder_code");

/// address of the liquidity pool we wish to provide liquidity to
pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");

/// ibc fee for the relayers
pub const IBC_FEE: Item<IbcFee> = Item::new("ibc_fee");
/// ibc transfer and ica timeouts that will be passed down to
/// contracts dealing with ICA
pub const TIMEOUTS: Item<Timeouts> = Item::new("timeouts");

/// fields related to the liquid staker module known prior to covenant instatiation.
/// remaining fields are filled and converted to an InstantiateMsg during the
/// instantiation chain.
pub const PRESET_LS_FIELDS: Item<covenant_ls::msg::PresetLsFields> = Item::new("preset_ls_fields");
/// fields related to the liquid pooler module known prior to covenant instatiation.
/// remaining fields are filled and converted to an InstantiateMsg during the
/// instantiation chain.
pub const PRESET_LP_FIELDS: Item<covenant_lp::msg::PresetLpFields> = Item::new("preset_lp_fields");
/// fields related to the depositor module known prior to covenant instatiation.
/// remaining fields are filled and converted to an InstantiateMsg during the
/// instantiation chain.
pub const PRESET_DEPOSITOR_FIELDS: Item<covenant_depositor::msg::PresetDepositorFields> =
    Item::new("preset_depositor_fields");
/// fields related to the clock contract known prior to covenant instatiation.
pub const PRESET_CLOCK_FIELDS: Item<covenant_clock::msg::PresetClockFields> =
    Item::new("preset_clock_fields");
/// fields related to the holder contract known prior to covenant instatiation.
pub const PRESET_HOLDER_FIELDS: Item<covenant_holder::msg::PresetHolderFields> =
    Item::new("preset_holder_fields");

/// address of the clock contract associated with this covenant
pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
/// address of the liquid pooler contract associated with this covenant
pub const COVENANT_LP_ADDR: Item<Addr> = Item::new("covenant_lp_addr");
/// address of the liquid staker contract associated with this covenant
pub const COVENANT_LS_ADDR: Item<Addr> = Item::new("covenant_ls_addr");
/// address of the depositor contract associated with this covenant
pub const COVENANT_DEPOSITOR_ADDR: Item<Addr> = Item::new("covenant_depositor_addr");
/// address of the holder contract associated with this covenant
pub const COVENANT_HOLDER_ADDR: Item<Addr> = Item::new("covenant_holder_addr");
