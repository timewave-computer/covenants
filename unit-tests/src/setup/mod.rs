use const_format::concatcp;
use cosmwasm_std::MemoryStorage;
use cw_multi_test::{addons::MockApiBech32, App, BankKeeper, WasmKeeper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use self::custom_module::NeutronKeeper;

pub mod base_suite;
pub mod contracts;
pub mod custom_module;
pub mod instantiates;
pub mod suite_builder;

pub type CustomApp = App<
    BankKeeper,
    MockApiBech32,
    MemoryStorage,
    NeutronKeeper,
    WasmKeeper<NeutronMsg, NeutronQuery>,
>;

// Denoms
pub const DENOM_FALLBACK: &str = "ufallback";
pub const DENOM_ATOM: &str = "uatom";
pub const DENOM_NTRN: &str = "untrn";
pub const DENOM_OSMO: &str = "uosmo";

/// This is used to fund the fuacet with all possible denoms we have.
/// so funding accounts and addresses can be done using the transfer msg
/// To fund the fuacet with another denom, just add the denom to the array
pub const ALL_DENOMS: &'static [&'static str] =
    &[DENOM_ATOM, DENOM_NTRN, DENOM_OSMO, DENOM_FALLBACK];

// Addrs
pub const FAUCET: &str = "faucet_addr";
pub const ADMIN: &str = "admin_addr";

// Salts for easier use (can append a number if more then 1 contract is needed)
pub const CLOCK_SALT: &str = "clock";
pub const SWAP_COVENANT_SALT: &str = "swap_covenant";
pub const SWAP_HOLDER_SALT: &str = "swap_holder";

// Channels between the chains
pub const NTRN_HUB_CHANNEL: (&str, &str) = ("channel-1", "channel-100");
pub const NTRN_OSMO_CHANNEL: (&str, &str) = ("channel-2", "channel-200");
pub const HUB_OSMO_CHANNEL: (&str, &str) = ("channel-3", "channel-300");

// IBC denoms

/// ntrn -> osmo
pub const DENOM_FALLBACK_ON_OSMO: &'static str =
    concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_FALLBACK);
pub const DENOM_FALLBACK_ON_HUB: &'static str = concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_FALLBACK);

/// hub -> ntrn
pub const DENOM_ATOM_ON_NTRN: &'static str = concatcp!(NTRN_HUB_CHANNEL.0, "/", DENOM_ATOM);
/// ntrn -> hub
pub const DENOM_NTRN_ON_HUB: &'static str = concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_NTRN);
/// osmo -> ntrn
pub const DENOM_OSMO_ON_NTRN: &'static str = concatcp!(NTRN_OSMO_CHANNEL.0, "/", DENOM_OSMO);
/// ntrn -> osmo
pub const DENOM_NTRN_ON_OSMO: &'static str = concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_NTRN);
/// osmo -> ntrn -> hub
pub const DENOM_OSMO_ON_HUB_FROM_NTRN: &'static str =
    concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_OSMO_ON_NTRN);
/// hub -> ntrn -> osmo
pub const DENOM_HUB_ON_OSMO_FROM_NTRN: &'static str =
    concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_ATOM_ON_NTRN);
