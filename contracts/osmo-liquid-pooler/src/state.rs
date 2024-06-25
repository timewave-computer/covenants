use cosmwasm_std::Addr;
use covenant_utils::op_mode::ContractOperationMode;
use cw_storage_plus::{Item, Map};

use crate::msg::{ContractState, IbcConfig, LiquidityProvisionConfig};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// operation mode of contract, either `Permissioned` or `Permissionless`
pub const CONTRACT_OP_MODE: Item<ContractOperationMode> = Item::new("contract_op_mode");

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

// timestamp to message
pub const POLYTONE_CALLBACKS: Map<String, String> = Map::new("callbacks");
