use cosmwasm_std::{BlockInfo, StdResult, Storage};
use cw_storage_plus::PrimaryKey;
use reversable_map::ReversableMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A first in, first out (FIFO) queue.
///
/// This reuses the technique from cw-wormhole to make the queue O(1)
/// in gas. See the wormhole essay for details:
///
/// <https://gist.github.com/0xekez/15fab6436ed593cbd59f0bdf7ecf1f61>
pub struct FIFOQueue<'a, T>(ReversableMap<'a, u64, T>);

impl<'a, T> FIFOQueue<'a, T>
where
    T: Serialize + DeserializeOwned + PrimaryKey<'a>,
{
    /// Enqueue's an element in the queue. The timestamp of the
    /// provided block is used as the elements entry time.
    ///
    /// - O(1) over the number of elements in the queue.
    /// - O(N) over the number of times that an element has been added
    ///   to the queue in the current block.
    pub fn enqueue(&self, storage: &mut dyn Storage, block: &BlockInfo, t: T) -> StdResult<()> {
        let mut time = block.time.nanos();
        while self.0.has(storage, time) {
            time = time + 1
        }
        self.0.save(storage, time, t)
    }

    /// Pops the oldest element from the queue and returns it, or if no
    /// elements are in the queue, returns None. O(1)
    pub fn dequeue(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        let Some((time, t)) = self
            .0
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .next()
            .transpose()? else {
		return Ok(None)
	    };
        self.0.remove(storage, time)?;
        Ok(Some(t))
    }

    /// Removes an element from the queue. Does not error if the
    /// element is already not in the queue. O(1)
    pub fn remove(&self, storage: &mut dyn Storage, t: T) -> StdResult<()> {
        self.0.reverse().remove(storage, t)
    }
}

mod reversable_map;
