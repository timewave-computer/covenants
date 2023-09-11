use cosmwasm_std::Addr;
use covenant_interchain_splitter::msg::PresetInterchainSplitterFields;
use covenant_swap_holder::msg::PresetSwapHolderFields;

use covenant_utils::SwapCovenantTerms;
use cw_storage_plus::Item;
use neutron_sdk::bindings::msg::IbcFee;

use crate::msg::{Timeouts, SwapCovenantParties};

/// contract code for the ibc forwarder
pub const IBC_FORWARDER_CODE: Item<u64> = Item::new("ibc_forwarder_code");
/// contract code for the interchain splitter
pub const INTERCHAIN_SPLITTER_CODE: Item<u64> = Item::new("interchain_splitter_code");
/// contract code for the swap holder
pub const SWAP_HOLDER_CODE: Item<u64> = Item::new("swap_holder_code");
/// contract code for the clock module
pub const CLOCK_CODE: Item<u64> = Item::new("clock_code");
/// contract code for the interchain router
pub const INTERCHAIN_ROUTER_CODE: Item<u64> = Item::new("interchain_router_code");

/// ibc fee for the relayers
pub const IBC_FEE: Item<IbcFee> = Item::new("ibc_fee");
/// ibc transfer and ica timeouts that will be passed down to
/// modules dealing with ICA
pub const TIMEOUTS: Item<Timeouts> = Item::new("timeouts");

// /// fields related to the clock module known prior to covenant instatiation.
pub const PRESET_CLOCK_FIELDS: Item<covenant_clock::msg::PresetClockFields> =
    Item::new("preset_clock_fields");

pub const PRESET_HOLDER_FIELDS: Item<PresetSwapHolderFields> = Item::new("preset_holder_fields");
pub const PRESET_SPLITTER_FIELDS: Item<PresetInterchainSplitterFields> = Item::new("preset_splitter_fields");


/// address of the clock module associated with this covenant
pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
/// address of the interchain splitter contract associated with this covenant
pub const COVENANT_INTERCHAIN_SPLITTER_ADDR: Item<Addr> = Item::new("covenant_interchain_splitter_addr");
/// address of the swap holder contract associated with this covenant
pub const COVENANT_SWAP_HOLDER_ADDR: Item<Addr> = Item::new("covenant_swap_holder_addr");


pub const COVENANT_PARTIES: Item<SwapCovenantParties> = Item::new("covenant_parties");
// pub const COVENANT_TERMS: Item<SwapCovenantTerms> = Item::new("swap_covenant_terms");

pub const PARTY_A_IBC_FORWARDER_ADDR: Item<Addr> = Item::new("party_a_ibc_forwarder_addr");
pub const PARTY_B_IBC_FORWARDER_ADDR: Item<Addr> = Item::new("party_b_ibc_forwarder_addr");

pub const PARTY_A_INTERCHAIN_ROUTER_ADDR: Item<Addr> = Item::new("party_a_interchain_router_addr");
pub const PARTY_B_INTERCHAIN_ROUTER_ADDR: Item<Addr> = Item::new("party_b_interchain_router_addr");