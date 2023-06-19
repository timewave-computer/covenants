use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Bound, Item, KeyDeserialize, PrimaryKey};
use reversable_map::ReversableMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A first in, first out (FIFO) queue.
///
/// This reuses the technique from cw-wormhole to make the queue O(1)
/// in gas. See the wormhole essay for details:
///
/// <https://gist.github.com/0xekez/15fab6436ed593cbd59f0bdf7ecf1f61>
///
/// This queue can hold a maximum of 2^64 items over its entire
/// lifetime. If more elements are added than that, an overflow panic
/// will occur. This constraint makes the design much simpler, and it
/// would cost a massive amount of gas to perform the number of
/// storage writes to make the counter overflow.
pub struct FIFOQueue<'a, T> {
    mapping: ReversableMap<'a, u64, T>,
    /// The number of items that have been added to the queue over its
    /// entire lifetime. Removing an item from the queue does not
    /// cause this to decrease.
    item_count: Item<'a, u64>,
}

impl<'a, T> FIFOQueue<'a, T>
where
    T: Serialize + DeserializeOwned + PrimaryKey<'a> + KeyDeserialize,
    T::Output: 'static,
{
    pub const fn new(
        forward_namespace: &'a str,
        reverse_namespace: &'a str,
        count_namespace: &'a str,
    ) -> Self {
        Self {
            mapping: ReversableMap::new(forward_namespace, reverse_namespace),
            item_count: Item::new(count_namespace),
        }
    }

    /// Enqueue's an element in the queue. The timestamp of the
    /// provided block is used as the elements entry time.
    ///
    /// - O(1) over the number of elements in the queue.
    /// - O(N) over the number of times that an element has been added
    ///   to the queue in the current block.
    ///
    /// Using the block timestamp isn't strictly nesecary (which is
    /// what gives us the O(N) case) (see cw-storage-plus's
    /// src/deque.rs), though it adds a fair bit of complexity.
    pub fn enqueue(&self, storage: &mut dyn Storage, t: T) -> StdResult<()> {
        let item_count = self.item_count.may_load(storage)?.unwrap_or_default();
        self.mapping.save(storage, item_count, t)?;
        self.item_count.save(storage, &(item_count + 1))
    }

    /// Pops the oldest element from the queue and returns it, or if no
    /// elements are in the queue, returns None. O(1)
    pub fn dequeue(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        let Some((time, t)) = self
            .mapping
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .next()
            .transpose()? else {
		return Ok(None)
	    };
        self.mapping.remove(storage, time)?;
        Ok(Some(t))
    }

    /// Removes an element from the queue. Does not error if the
    /// element is already not in the queue. O(1)
    pub fn remove(&self, storage: &mut dyn Storage, t: T) -> StdResult<()> {
        self.mapping.reverse().remove(storage, t)
    }

    /// Returns true if `t` is in the queue, false otherwise.
    pub fn has(&self, storage: &dyn Storage, t: T) -> bool {
        self.mapping.reverse().has(storage, t)
    }

    pub fn query_queue(
        &self,
        storage: &dyn Storage,
        start_after: Option<T>,
        limit: Option<u32>,
    ) -> StdResult<Vec<(T::Output, u64)>> {
        let range = self.mapping.reverse().range(
            storage,
            start_after.map(|t| Bound::exclusive(t)),
            None,
            cosmwasm_std::Order::Ascending,
        );
        match limit {
            None => range.collect::<StdResult<_>>(),
            Some(limit) => range.take(limit as usize).collect::<StdResult<_>>(),
        }
    }
}

mod reversable_map;
#[cfg(test)]
mod tests;
