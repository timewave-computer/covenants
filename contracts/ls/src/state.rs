use cosmwasm_std::{from_binary, to_vec, Addr, Binary, Order, StdResult, Storage, Uint64};
use cw_storage_plus::{Item, Map};
use neutron_sdk::bindings::msg::IbcFee;

use crate::msg::{AcknowledgementResult, ContractState, SudoPayload};

/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the incoming ticks sender
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// liquid pooler module address to forward the liquid staked funds to
pub const LP_ADDRESS: Item<Addr> = Item::new("lp_address");

/// IBC transfer channel on stride for neutron
pub const STRIDE_NEUTRON_IBC_TRANSFER_CHANNEL_ID: Item<String> = Item::new("sn_ibc_chann_id");
/// IBC connection ID on neutron for stride
pub const NEUTRON_STRIDE_IBC_CONNECTION_ID: Item<String> = Item::new("ns_ibc_conn_id");

/// the denom that we will permit transfers of to the liquid pooler
pub const LS_DENOM: Item<String> = Item::new("ls_denom");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");

/// timeout in seconds for inner ibc MsgTransfer
pub const IBC_TRANSFER_TIMEOUT: Item<Uint64> = Item::new("ibc_transfer_timeout");
/// time in seconds for ICA SubmitTX messages from neutron
pub const ICA_TIMEOUT: Item<Uint64> = Item::new("ica_timeout");
/// neutron IbcFee for relayers
pub const IBC_FEE: Item<IbcFee> = Item::new("ibc_fee");

/// interchain transaction responses - ack/err/timeout state to query later
pub const ACKNOWLEDGEMENT_RESULTS: Map<(String, u64), AcknowledgementResult> =
    Map::new("acknowledgement_results");
pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");
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
