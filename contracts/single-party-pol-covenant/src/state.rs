use cosmwasm_std::Addr;
use covenant_astroport_liquid_pooler::msg::PresetAstroLiquidPoolerFields;
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;

use covenant_interchain_splitter::msg::PresetInterchainSplitterFields;
use covenant_two_party_pol_holder::msg::PresetTwoPartyPolHolderFields;
use cw_storage_plus::Item;

// fields related to the contracts known prior to their.
pub const PRESET_CLOCK_FIELDS: Item<PresetClockFields> = Item::new("preset_clock_fields");
pub const PRESET_HOLDER_FIELDS: Item<PresetTwoPartyPolHolderFields> =
    Item::new("preset_holder_fields");
pub const PRESET_SPLITTER_FIELDS: Item<PresetInterchainSplitterFields> =
    Item::new("preset_splitter_fields");
pub const PRESET_FORWARDER_A_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_forwarder_a_fields");
pub const PRESET_FORWARDER_B_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_forwarder_b_fields");
pub const PRESET_LIQUID_POOLER_FIELDS: Item<PresetAstroLiquidPoolerFields> =
    Item::new("preset_lp_fields");
// pub const PRESET_LIQUID_STAKER_FIELDS: Item<PresetAstroLiquidPoolerFields> =
// Item::new("preset_ls_fields");

pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const HOLDER_ADDR: Item<Addr> = Item::new("holder_addr");
pub const SPLITTER_ADDR: Item<Addr> = Item::new("ibc_splitter_addr");
pub const IBC_FORWARDER_A_ADDR: Item<Addr> = Item::new("ibc_forwarder_a_addr");
pub const IBC_FORWARDER_B_ADDR: Item<Addr> = Item::new("ibc_forwarder_b_addr");
pub const LIQUID_POOLER_ADDR: Item<Addr> = Item::new("liquid_pooler_addr");
pub const LIQUID_STAKER_ADDR: Item<Addr> = Item::new("liquid_staker_addr");
