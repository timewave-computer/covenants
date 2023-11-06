use cosmwasm_std::Addr;
use covenant_astroport_liquid_pooler::msg::PresetAstroLiquidPoolerFields;
use covenant_clock::msg::PresetClockFields;
use covenant_remote_chain_splitter::msg::PresetRemoteChainSplitterFields;
use covenant_stride_liquid_staker::msg::PresetLsFields;
use cw_storage_plus::Item;


pub const PRESET_LS_FIELDS: Item<PresetLsFields> = Item::new("preset_ls_fields");
pub const PRESET_LP_FIELDS: Item<PresetAstroLiquidPoolerFields> = Item::new("preset_lp_fields");
pub const PRESET_CLOCK_FIELDS: Item<PresetClockFields> =
    Item::new("preset_clock_fields");
// pub const PRESET_HOLDER_FIELDS: Item<PresetClockFields> =
//     Item::new("preset_holder_fields");
pub const PRESET_REMOTE_CHAIN_SPLITTER_FIELDS: Item<PresetRemoteChainSplitterFields> =
    Item::new("preset_remote_chain_splitter_fields");

/// address of the clock module associated with this covenant
pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
/// address of the liquid pooler module associated with this covenant
pub const COVENANT_LP_ADDR: Item<Addr> = Item::new("covenant_lp_addr");
/// address of the liquid staker module associated with this covenant
pub const COVENANT_LS_ADDR: Item<Addr> = Item::new("covenant_ls_addr");
/// address of the depositor module associated with this covenant
pub const COVENANT_DEPOSITOR_ADDR: Item<Addr> = Item::new("covenant_depositor_addr");
/// address of the holder module associated with this covenant
pub const COVENANT_HOLDER_ADDR: Item<Addr> = Item::new("covenant_holder_addr");
pub const COVENANT_REMOTE_CHAIN_SPLITTER_ADDR: Item<Addr> = Item::new("covenant_remote_chain_splitter");
