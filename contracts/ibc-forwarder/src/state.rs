use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::{Item, Map};
use neutron_sdk::bindings::msg::IbcFee;

use crate::msg::{ContractState, RemoteChainInfo};



/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

/// information needed for an ibc transfer to the remote chain
pub const REMOTE_CHAIN_INFO: Item<RemoteChainInfo> = Item::new("r_c_info");

/// timeout in seconds for inner ibc MsgTransfer
pub const IBC_TRANSFER_TIMEOUT: Item<Uint64> = Item::new("ibc_transfer_timeout");
/// time in seconds for ICA SubmitTX messages from neutron
pub const ICA_TIMEOUT: Item<Uint64> = Item::new("ica_timeout");
/// neutron IbcFee for relayers
pub const IBC_FEE: Item<IbcFee> = Item::new("ibc_fee");


/// id of the connection between neutron and remote chain on which we
/// wish to open an ICA on
// pub const REMOTE_CHAIN_CONNECTION_ID: Item<String> = Item::new("rc_conn_id");
// pub const REMOTE_CHAIN_DENOM: Item<String> = Item::new("rc_denom");
// pub const TRANSFER_CHANNEL_ID: Item<String> = Item::new("transfer_chann_id");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> = Map::new("interchain_accounts");

pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");
