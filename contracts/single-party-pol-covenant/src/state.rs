use cosmwasm_std::Addr;
use covenant_astroport_liquid_pooler::msg::PresetAstroLiquidPoolerFields;
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;

use covenant_native_splitter::msg::PresetNativeSplitterFields;
use covenant_single_party_pol_holder::msg::PresetHolderFields;
use covenant_stride_liquid_staker::msg::PresetStrideLsFields;
use cw_storage_plus::Item;

// fields related to the contracts known prior to their.
pub const PRESET_CLOCK_FIELDS: Item<PresetClockFields> = Item::new("preset_clock_fields");
pub const PRESET_HOLDER_FIELDS: Item<PresetHolderFields> = Item::new("preset_holder_fields");
pub const PRESET_SPLITTER_FIELDS: Item<PresetNativeSplitterFields> =
    Item::new("preset_splitter_fields");
pub const PRESET_LS_FORWARDER_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_ls_forwarder_fields");
pub const PRESET_LP_FORWARDER_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_lp_forwarder_fields");
pub const PRESET_LIQUID_POOLER_FIELDS: Item<PresetAstroLiquidPoolerFields> =
    Item::new("preset_lp_fields");
pub const PRESET_LIQUID_STAKER_FIELDS: Item<PresetStrideLsFields> = Item::new("preset_ls_fields");

pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const HOLDER_ADDR: Item<Addr> = Item::new("holder_addr");
pub const SPLITTER_ADDR: Item<Addr> = Item::new("remote_chain_splitter_addr");
pub const LIQUID_POOLER_ADDR: Item<Addr> = Item::new("liquid_pooler_addr");
pub const LIQUID_STAKER_ADDR: Item<Addr> = Item::new("liquid_staker_addr");
pub const LS_FORWARDER_ADDR: Item<Addr> = Item::new("ls_forwarder_addr");
pub const LP_FORWARDER_ADDR: Item<Addr> = Item::new("lp_forwarder_addr");
pub const ROUTER_ADDR: Item<Addr> = Item::new("router_addr");
