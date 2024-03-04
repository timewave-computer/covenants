use const_format::concatcp;
use cosmwasm_std::MemoryStorage;
use cw_multi_test::{addons::MockApiBech32, App, BankKeeper, WasmKeeper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use self::custom_module::NeutronKeeper;

pub mod astro_contracts;
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

// TODO: Notes
// 1. The ls/lp forwader config in the single party is really confusing, because you only use like half of the fields in the actual contract, I think a special config
//    that is needed.
// 2. forwarder config is very confusing, local means the receiving end, while remote means the sending end, while this makes sense for when the forwarder
//    forwards funds to neutron (so neutron is local in this case), it doesn't when it comes to ls config, where local is stride and remote is atom.

// Denoms
pub const DENOM_FALLBACK: &str = "ufallback";
pub const DENOM_ATOM: &str = "uatom";
pub const DENOM_NTRN: &str = "untrn";
pub const DENOM_OSMO: &str = "uosmo";

/// This is used to fund the fuacet with all possible denoms we have.
/// so funding accounts and addresses can be done using the transfer msg
/// To fund the fuacet with another denom, just add the denom to the array
pub const ALL_DENOMS: &[&str] = &[
    DENOM_ATOM,
    DENOM_NTRN,
    DENOM_OSMO,
    DENOM_FALLBACK,
    DENOM_ATOM_ON_NTRN,
    DENOM_LS_ATOM_ON_NTRN,
];

// Addrs
pub const FAUCET: &str = "faucet_addr";
pub const ADMIN: &str = "admin_addr";

// Salts for easier use (can append a number if more then 1 contract is needed)
pub const CLOCK_SALT: &str = "clock";
pub const SWAP_COVENANT_SALT: &str = "swap_covenant";
pub const SINGLE_PARTY_COVENANT_SALT: &str = "single_party_covenant";
pub const SWAP_HOLDER_SALT: &str = "swap_holder";
pub const TWO_PARTY_HOLDER_SALT: &str = "two_party_holder";
pub const SINGLE_PARTY_HOLDER_SALT: &str = "single_party_holder";
pub const ASTRO_LIQUID_POOLER_SALT: &str = "astro_liquid_pooler";
pub const NATIVE_SPLITTER_SALT: &str = "native_splitter";
pub const REMOTE_CHAIN_SPLITTER_SALT: &str = "remote_chain_splitter";
pub const INTERCHAIN_ROUTER_SALT: &str = "interchain_router";
pub const NATIVE_ROUTER_SALT: &str = "native_router";
pub const IBC_FORWARDER_SALT: &str = "ibc_forwarder";

// Channels between the chains
pub const NTRN_HUB_CHANNEL: (&str, &str) = ("channel-1", "channel-100");
pub const NTRN_OSMO_CHANNEL: (&str, &str) = ("channel-2", "channel-200");
pub const HUB_OSMO_CHANNEL: (&str, &str) = ("channel-3", "channel-300");
pub const HUB_STRIDE_CHANNEL: (&str, &str) = ("channel-4", "channel-400");
pub const NTRN_STRIDE_CHANNEL: (&str, &str) = ("channel-5", "channel-500");

// IBC denoms

/// ntrn -> osmo
pub const DENOM_FALLBACK_ON_OSMO: &str = concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_FALLBACK);
pub const DENOM_FALLBACK_ON_HUB: &str = concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_FALLBACK);

// LS tokens on stride
/// lsATOM on stride
pub const DENOM_LS_ATOM_ON_STRIDE: &str = concatcp!(HUB_STRIDE_CHANNEL.1, "/", DENOM_ATOM);

/// hub -> ntrn
pub const DENOM_ATOM_ON_NTRN: &str = concatcp!(NTRN_HUB_CHANNEL.0, "/", DENOM_ATOM);
/// ntrn -> hub
pub const DENOM_NTRN_ON_HUB: &str = concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_NTRN);
/// osmo -> ntrn
pub const DENOM_OSMO_ON_NTRN: &str = concatcp!(NTRN_OSMO_CHANNEL.0, "/", DENOM_OSMO);
/// ntrn -> osmo
pub const DENOM_NTRN_ON_OSMO: &str = concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_NTRN);
/// lsATOM -> ntrn
pub const DENOM_LS_ATOM_ON_NTRN: &str =
    concatcp!(NTRN_STRIDE_CHANNEL.0, "/", DENOM_LS_ATOM_ON_STRIDE);
/// osmo -> ntrn -> hub
pub const DENOM_OSMO_ON_HUB_FROM_NTRN: &str =
    concatcp!(NTRN_HUB_CHANNEL.1, "/", DENOM_OSMO_ON_NTRN);
/// hub -> ntrn -> osmo
pub const DENOM_HUB_ON_OSMO_FROM_NTRN: &str =
    concatcp!(NTRN_OSMO_CHANNEL.1, "/", DENOM_ATOM_ON_NTRN);
