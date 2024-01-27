use crate::msg::CovenantContractCodeIds;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const HOLDER_ADDR: Item<Addr> = Item::new("holder_addr");
pub const SPLITTER_ADDR: Item<Addr> = Item::new("remote_chain_splitter_addr");
pub const LIQUID_POOLER_ADDR: Item<Addr> = Item::new("liquid_pooler_addr");
pub const LIQUID_STAKER_ADDR: Item<Addr> = Item::new("liquid_staker_addr");
pub const LS_FORWARDER_ADDR: Item<Addr> = Item::new("ls_forwarder_addr");
pub const LP_FORWARDER_ADDR: Item<Addr> = Item::new("lp_forwarder_addr");
pub const ROUTER_ADDR: Item<Addr> = Item::new("router_addr");

pub const CONTRACT_CODES: Item<CovenantContractCodeIds> = Item::new("contract_codes");
