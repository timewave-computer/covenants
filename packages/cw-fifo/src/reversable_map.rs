use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

/// A bidirectional map.
#[derive(Debug, Clone)]
pub struct ReversableMap<'a, A, B>(Map<'a, A, B>, Map<'a, B, A>);

impl<'a, A, B> ReversableMap<'a, A, B> {
    pub const fn new(forward_namespace: &'a str, reverse_namespace: &'a str) -> Self {
        Self(Map::new(forward_namespace), Map::new(reverse_namespace))
    }
}

impl<'a, A, B> ReversableMap<'a, A, B>
where
    // I think the compiler is being too strict by requiring these
    // clone bounds. Map holds PhantomData of types A and B, so should
    // not require them to be cloneable.
    A: Clone,
    B: Clone,
{
    pub fn reverse(&self) -> ReversableMap<'a, B, A> {
        // Clone is cheap as `Map` is only a pointer to a namespace.
        ReversableMap(self.1.clone(), self.0.clone())
    }
}

impl<'a, A, B> ReversableMap<'a, A, B>
where
    A: Serialize + DeserializeOwned + PrimaryKey<'a>,
    B: Serialize + DeserializeOwned + PrimaryKey<'a>,
{
    pub fn has(&self, s: &dyn Storage, k: A) -> bool {
        self.0.has(s, k)
    }

    pub fn remove(&self, s: &mut dyn Storage, k: A) -> StdResult<()> {
        let mapping = self.load(s, k.clone())?;
        self.1.remove(s, mapping);
        self.0.remove(s, k);
        Ok(())
    }

    pub fn save(&self, s: &mut dyn Storage, k: A, v: B) -> StdResult<()> {
        if let Some(old) = self.0.may_load(s, k.clone())? {
            // If there was previously a value set in the map, remove
            // the old mapping. For example, given a map:
            //
            // 10 <-> 11
            //
            // Setting 10 to a new value should remove the old value.
            self.1.remove(s, old)
        }
        self.0.save(s, k.clone(), &v)?;
        self.1.save(s, v, &k)
    }

    pub fn load(&self, s: &dyn Storage, k: A) -> StdResult<B> {
        self.0.load(s, k)
    }
}

impl<'a, A, B> ReversableMap<'a, A, B>
where
    A: PrimaryKey<'a> + KeyDeserialize,
    B: Serialize + DeserializeOwned,
{
    pub fn range<'c>(
        &self,
        s: &'c dyn Storage,
        min: Option<Bound<'a, A>>,
        max: Option<Bound<'a, A>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(A::Output, B)>> + 'c>
    where
        B: 'c,
        A::Output: 'static,
    {
        self.0.range(s, min, max, order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_save_bidirectional() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;

        let rm = ReversableMap::new("front", "back");
        rm.save(storage, 10, 11).unwrap();

        assert_eq!(rm.reverse().load(storage, 11).unwrap(), 10);

        rm.reverse().save(storage, 11, 9).unwrap();

        assert!(!rm.has(storage, 10));
        assert_eq!(rm.load(storage, 9).unwrap(), 11)
    }

    #[test]
    fn test_remove_bidirectional() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;

        let rm = ReversableMap::new("front", "back");
        rm.save(storage, 10, 11).unwrap();
        rm.save(storage, 8, 9).unwrap();

        rm.remove(storage, 8).unwrap();
        rm.reverse().remove(storage, 11).unwrap();

        assert_eq!(rm.range(storage, None, None, Order::Ascending).next(), None);
    }
}
