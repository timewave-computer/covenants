use cosmwasm_std::Addr;
use covenant_clock::msg::PresetClockFields;
use covenant_ibc_forwarder::msg::PresetIbcForwarderFields;

use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_two_party_pol_holder::msg::PresetTwoPartyPolHolderFields;
use cw_storage_plus::Item;

// fields related to the contracts known prior to their.
pub const PRESET_CLOCK_FIELDS: Item<PresetClockFields> = Item::new("preset_clock_fields");
pub const PRESET_HOLDER_FIELDS: Item<PresetTwoPartyPolHolderFields> = Item::new("preset_holder_fields");
pub const PRESET_PARTY_A_FORWARDER_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_party_a_forwarder_fields");
pub const PRESET_PARTY_B_FORWARDER_FIELDS: Item<PresetIbcForwarderFields> =
    Item::new("preset_party_b_forwarder_fields");
pub const PRESET_PARTY_A_ROUTER_FIELDS: Item<PresetInterchainRouterFields> =
    Item::new("preset_party_a_router_fields");
pub const PRESET_PARTY_B_ROUTER_FIELDS: Item<PresetInterchainRouterFields> =
    Item::new("preset_party_b_router_fields");

pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const COVENANT_POL_HOLDER_ADDR: Item<Addr> = Item::new("covenant_two_party_pol_holder_addr");
pub const PARTY_A_IBC_FORWARDER_ADDR: Item<Addr> = Item::new("party_a_ibc_forwarder_addr");
pub const PARTY_B_IBC_FORWARDER_ADDR: Item<Addr> = Item::new("party_b_ibc_forwarder_addr");
