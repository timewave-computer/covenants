use cosmwasm_std::{from_json, to_json_vec, Addr, Binary, StdError, StdResult, Storage};
use covenant_utils::{
    ica::IcaStateHelper,
    neutron::{RemoteChainInfo, SudoPayload},
};
use cw_storage_plus::{Item, Map};

use crate::msg::ContractState;

/// tracks the current state of state machine
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");

/// clock module address to verify the sender of incoming ticks
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// next contract address to forward the liquid staked funds to
pub const NEXT_CONTRACT: Item<Addr> = Item::new("next_contract");

/// information needed for an ibc transfer to the remote chain
pub const REMOTE_CHAIN_INFO: Item<RemoteChainInfo> = Item::new("r_c_info");

/// interchain accounts storage in form of (port_id) -> (address, controller_connection_id)
pub const INTERCHAIN_ACCOUNTS: Map<String, Option<(String, String)>> =
    Map::new("interchain_accounts");

/// interchain transaction responses - ack/err/timeout state to query later
pub const REPLY_ID_STORAGE: Item<Vec<u8>> = Item::new("reply_queue_id");
pub const SUDO_PAYLOAD: Map<(String, u64), Vec<u8>> = Map::new("sudo_payload");

pub(crate) struct LiquidStakerIcaStateHelper;

impl IcaStateHelper for LiquidStakerIcaStateHelper {
    fn reset_state(&self, storage: &mut dyn Storage) -> StdResult<()> {
        CONTRACT_STATE.save(storage, &ContractState::Instantiated)?;
        Ok(())
    }

    fn clear_ica(&self, storage: &mut dyn Storage) -> StdResult<()> {
        INTERCHAIN_ACCOUNTS.clear(storage);
        Ok(())
    }

    fn save_ica(
        &self,
        storage: &mut dyn Storage,
        port_id: String,
        address: String,
        controller_connection_id: String,
    ) -> StdResult<()> {
        INTERCHAIN_ACCOUNTS.save(storage, port_id, &Some((address, controller_connection_id)))?;
        Ok(())
    }

    fn save_state_ica_created(&self, storage: &mut dyn Storage) -> StdResult<()> {
        CONTRACT_STATE.save(storage, &ContractState::IcaCreated)?;
        Ok(())
    }

    fn save_reply_payload(&self, storage: &mut dyn Storage, payload: SudoPayload) -> StdResult<()> {
        REPLY_ID_STORAGE.save(storage, &to_json_vec(&payload)?)?;
        Ok(())
    }

    fn read_reply_payload(&self, storage: &mut dyn Storage) -> StdResult<SudoPayload> {
        let data = REPLY_ID_STORAGE.load(storage)?;
        from_json(Binary(data))
    }

    fn save_sudo_payload(
        &self,
        storage: &mut dyn Storage,
        channel_id: String,
        seq_id: u64,
        payload: SudoPayload,
    ) -> StdResult<()> {
        SUDO_PAYLOAD.save(storage, (channel_id, seq_id), &to_json_vec(&payload)?)
    }

    fn get_ica(&self, storage: &dyn Storage, key: String) -> StdResult<(String, String)> {
        INTERCHAIN_ACCOUNTS
            .load(storage, key)?
            .ok_or_else(|| StdError::generic_err("Interchain account is not created yet"))
    }
}
