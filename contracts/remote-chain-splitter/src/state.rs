use cosmwasm_std::{from_json, to_json_vec, Addr, Binary, Order, StdResult, Storage, Uint128};
use covenant_utils::{neutron::{RemoteChainInfo, SudoPayload}, split::SplitConfig};
use cw_storage_plus::{Item, Map};

use crate::msg::ContractState;

/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

pub const TRANSFER_AMOUNT: Item<Uint128> = Item::new("transfer_amount");

// maps a denom string to a vec of SplitReceivers
pub const SPLIT_CONFIG_MAP: Map<String, SplitConfig> = Map::new("split_config");

/// information needed for an ibc transfer to the remote chain
pub const REMOTE_CHAIN_INFO: Item<RemoteChainInfo> = Item::new("r_c_info");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");

pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");
pub const ERRORS_QUEUE: Map<u32, String> = Map::new("errors_queue");

pub fn save_reply_payload(store: &mut dyn Storage, payload: SudoPayload) -> StdResult<()> {
    REPLY_ID_STORAGE.save(store, &to_json_vec(&payload)?)
}

pub fn read_reply_payload(store: &mut dyn Storage) -> StdResult<SudoPayload> {
    let data = REPLY_ID_STORAGE.load(store)?;
    from_json(Binary(data))
}

pub fn add_error_to_queue(store: &mut dyn Storage, error_msg: String) -> Option<()> {
    let result = ERRORS_QUEUE
        .keys(store, None, None, Order::Descending)
        .next()
        .and_then(|data| data.ok())
        .map(|c| c + 1)
        .or(Some(0));

    result.and_then(|idx| ERRORS_QUEUE.save(store, idx, &error_msg).ok())
}

pub fn read_errors_from_queue(store: &dyn Storage) -> StdResult<Vec<(Vec<u8>, String)>> {
    ERRORS_QUEUE
        .range_raw(store, None, None, Order::Ascending)
        .collect()
}

pub fn read_sudo_payload(
    store: &mut dyn Storage,
    channel_id: String,
    seq_id: u64,
) -> StdResult<SudoPayload> {
    let data = SUDO_PAYLOAD.load(store, (channel_id, seq_id))?;
    from_json(Binary(data))
}

pub fn save_sudo_payload(
    store: &mut dyn Storage,
    channel_id: String,
    seq_id: u64,
    payload: SudoPayload,
) -> StdResult<()> {
    SUDO_PAYLOAD.save(store, (channel_id, seq_id), &to_json_vec(&payload)?)
}

pub fn clear_sudo_payload(store: &mut dyn Storage, channel_id: String, seq_id: u64) {
    SUDO_PAYLOAD.remove(store, (channel_id, seq_id))
}
