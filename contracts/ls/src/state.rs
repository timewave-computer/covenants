use cosmwasm_std::{from_binary, to_vec, Addr, Binary, Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// store the clock address to verify calls
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");

// the ibc transfer channel
pub const STRIDE_NEUTRON_IBC_TRANSFER_CHANNEL_ID: Item<String> = Item::new("sn_ibc_chann_id");
pub const NEUTRON_STRIDE_IBC_CONNECTION_ID: Item<String> = Item::new("ns_ibc_conn_id");
pub const LP_ADDRESS: Item<String> = Item::new("lp_address");
pub const ICA_ADDRESS: Item<String> = Item::new("ica_address");
pub const LS_DENOM: Item<String> = Item::new("ls_denom");

pub const IBC_MSG_TRANSFER_TIMEOUT_TIMESTAMP: Item<u64> = Item::new("timeout");

// ICA
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");
pub const IBC_PORT_ID: Item<String> = Item::new("ibc_port_id");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Instantiated,
    ICACreated,
}

pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SudoPayload {
    pub message: String,
    pub port_id: String,
}

pub const SUDO_PAYLOAD_REPLY_ID: u64 = 1;

pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");

// interchain transaction responses - ack/err/timeout state to query later
pub const ACKNOWLEDGEMENT_RESULTS: Map<(String, u64), AcknowledgementResult> =
    Map::new("acknowledgement_results");

pub const ERRORS_QUEUE: Map<u32, String> = Map::new("errors_queue");

/// Serves for storing acknowledgement calls for interchain transactions
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AcknowledgementResult {
    /// Success - Got success acknowledgement in sudo with array of message item types in it
    Success(Vec<String>),
    /// Error - Got error acknowledgement in sudo with payload message in it and error details
    Error((String, String)),
    /// Timeout - Got timeout acknowledgement in sudo with payload message in it
    Timeout(String),
}

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
