use cosmwasm_std::{Addr, Uint128};
use covenant_utils::neutron::RemoteChainInfo;
use cw_storage_plus::{Item, Map};

use crate::msg::ContractState;

/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const TRANSFER_AMOUNT: Item<Uint128> = Item::new("transfer_amount");

pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

/// information needed for an ibc transfer to the remote chain
pub const REMOTE_CHAIN_INFO: Item<RemoteChainInfo> = Item::new("r_c_info");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");

pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");
pub const FALLBACK_ADDRESS: Item<String> = Item::new("fallback_address");
