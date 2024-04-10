use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::Binary;
use cosmwasm_std::Uint64;
use covenant_clock_derive::clocked;

#[cw_serde]
pub struct InstantiateMsg {
    /// The max gas that may be used by a tick receiver. If more than
    /// this amount of gas is used, the tick will be treated as if it
    /// errored, and it will be sent to the back of the queue.
    ///
    /// At the most, this should be no larger than 100_000 gas less
    /// the chain's block max gas. This overhead is needed so the
    /// clock always has enough gas after executing the tick to handle
    /// its failure.
    ///
    /// This value may be updated later by the contract admin.
    pub tick_max_gas: Option<Uint64>,
    /// Whitelist of contracts that are allowed to be queued and ticked
    pub whitelist: Vec<String>,
}

#[cw_serde]
pub struct PresetClockFields {
    pub tick_max_gas: Option<Uint64>,
    pub whitelist: Vec<String>,
    pub clock_code: u64,
    pub label: String,
}

impl PresetClockFields {
    pub fn to_instantiate_msg(self) -> InstantiateMsg {
        let tick_max_gas = if let Some(tmg) = self.tick_max_gas {
            // double the 100k minimum seems fair
            tmg.min(Uint64::new(200000))
        } else {
            // todo: find some reasonable default value
            Uint64::new(2900000)
        };

        InstantiateMsg {
            tick_max_gas: Some(tick_max_gas),
            whitelist: self.whitelist,
        }
    }
}

#[clocked] // Adds a `Tick {}` message which can be called permissionlessly to advance the clock.
#[cw_serde]
pub enum ExecuteMsg {
    /// Enqueues the message sender for ticks (serialized as messages
    /// in the form `{"tick": {}}`). The sender will continue to
    /// receive ticks until sending a `Dequeue {}` message. Only
    /// callable if the message sender is not currently enqueued and
    /// is a contract.
    Enqueue {},
    /// Dequeues the message sender stopping them from receiving
    /// ticks. Only callable if the message sender is currently
    /// enqueued.
    Dequeue {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns true if `address` is in the queue, and false
    /// otherwise.
    #[returns(bool)]
    IsQueued { address: String },
    /// Paginated query for all the elements in the queue. Returns
    /// elements in asending order by address in the form (address,
    /// timestamp) where timestamp is the nanosecond unix timestamp at
    /// which address was added to the queue.
    #[returns(Vec<(Addr, u64)>)]
    Queue {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Queries the current tick max gas, as set during instantiation
    /// and updated via migration.
    #[returns(Uint64)]
    TickMaxGas {},
    /// Queries if the contract is paused.
    #[returns(bool)]
    Paused {},

    /// Queries if the contract is paused.
    #[returns(Vec<Addr>)]
    Whitelist {},
}

#[cw_serde]
pub enum MigrateMsg {
    /// Pauses the clock. No `ExecuteMsg` messages will be executable
    /// until the clock is unpaused. Callable only if the clock is
    /// unpaused.
    Pause {},
    /// Unpauses the clock. Callable only if the clock is paused.
    Unpause {},
    /// Updates the max gas allowed to be consumed by a tick. This
    /// should be no larger than 100_000 less the block max gas so as
    /// to save enough gas to process the tick's error.
    UpdateTickMaxGas {
        new_value: Uint64,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
    ManageWhitelist {
        add: Option<Vec<String>>,
        remove: Option<Vec<String>>,
    },
}
