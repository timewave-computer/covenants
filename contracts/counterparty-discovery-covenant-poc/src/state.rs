use crate::msg::CovenantContractCodes;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const COVENANT_CLOCK_ADDR: Item<Addr> = Item::new("covenant_clock_addr");
pub const COVENANT_POL_HOLDER_ADDR: Item<Addr> = Item::new("covenant_two_party_pol_holder_addr");
pub const LIQUID_POOLER_ADDR: Item<Addr> = Item::new("liquid_pooler_addr");

pub(crate) const CONTRACT_CODES: Item<CovenantContractCodes> = Item::new("contract_codes");
