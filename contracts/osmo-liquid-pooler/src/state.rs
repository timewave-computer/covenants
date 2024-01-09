use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::{ContractState, LiquidityProvisionConfig, IbcConfig};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the incoming ticks sender
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

/// holder module address to verify withdrawal requests
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");

// polytone note address
pub const NOTE_ADDRESS: Item<Addr> = Item::new("note_address");
// our address on osmosis created by polytone
pub const PROXY_ADDRESS: Item<String> = Item::new("proxy_address");

// fields relevant for providing liquidity
pub const LIQUIDITY_PROVISIONING_CONFIG: Item<LiquidityProvisionConfig> = Item::new("lp_config");

// ibc-related fields
pub const IBC_CONFIG: Item<IbcConfig> = Item::new("ibc_config");

