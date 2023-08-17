use crate::msg::{AcknowledgementResult, ContractState, IbcConfig, SudoPayload, WeightedReceiver};
use cosmwasm_std::{from_binary, to_vec, Addr, Binary, Order, StdResult, Storage, Timestamp};
use cw_storage_plus::{Item, Map};

/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// liquid staker module address to query the stride ICA address to autopilot to
pub const LS_ADDRESS: Item<Addr> = Item::new("ls_address");
/// liquid pooler module address to forward the native tokens to
pub const LP_ADDRESS: Item<Addr> = Item::new("lp_address");

/// formatting of stride autopilot message.
/// we use string match & replace with relevant fields to obtain the valid message.
pub const AUTOPILOT_FORMAT: Item<String> = Item::new("autopilot_format");

/// addr and amount of atom to liquid stake on stride
pub const STRIDE_ATOM_RECEIVER: Item<WeightedReceiver> = Item::new("stride_atom_receiver");
/// addr and amount of atom
pub const NATIVE_ATOM_RECEIVER: Item<WeightedReceiver> = Item::new("native_atom_receiver");

/// ibc denom of atom on neutron
pub const NEUTRON_ATOM_IBC_DENOM: Item<String> = Item::new("neutron_atom_ibc_denom");

/// neutron ibc transfer channel id on gaia
pub const GAIA_NEUTRON_IBC_TRANSFER_CHANNEL_ID: Item<String> = Item::new("gn_ibc_chann_id");
/// stride ibc transfer channel id on gaia
pub const GAIA_STRIDE_IBC_TRANSFER_CHANNEL_ID: Item<String> = Item::new("gs_ibc_chan_id");
/// connection id of gaia on neutron
pub const NEUTRON_GAIA_CONNECTION_ID: Item<String> = Item::new("ng_conn_id");

/// config containing ibc fee, ica timeout, and ibc transfer
pub const IBC_CONFIG: Item<IbcConfig> = Item::new("ibc_config");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");

// pending transaction timeout timestamp
pub const PENDING_NATIVE_TRANSFER_TIMEOUT: Item<Timestamp> =
    Item::new("pending_native_transfer_timeout");

pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");

// interchain transaction responses - ack/err/timeout state to query later
pub const ACKNOWLEDGEMENT_RESULTS: Map<(String, u64), AcknowledgementResult> =
    Map::new("acknowledgement_results");

pub const ERRORS_QUEUE: Map<u32, String> = Map::new("errors_queue");

pub fn save_reply_payload(store: &mut dyn Storage, payload: SudoPayload) -> StdResult<()> {
    REPLY_ID_STORAGE.save(store, &to_vec(&payload)?)
}

pub fn read_reply_payload(store: &mut dyn Storage) -> StdResult<SudoPayload> {
    let data = REPLY_ID_STORAGE.load(store)?;
    from_binary(&Binary(data))
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
    from_binary(&Binary(data))
}

pub fn save_sudo_payload(
    store: &mut dyn Storage,
    channel_id: String,
    seq_id: u64,
    payload: SudoPayload,
) -> StdResult<()> {
    SUDO_PAYLOAD.save(store, (channel_id, seq_id), &to_vec(&payload)?)
}

pub fn clear_sudo_payload(store: &mut dyn Storage, channel_id: String, seq_id: u64) {
    SUDO_PAYLOAD.remove(store, (channel_id, seq_id))
}
