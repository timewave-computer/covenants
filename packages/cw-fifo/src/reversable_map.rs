use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

/// A bidirectional map.
pub struct ReversableMap<'a, A, B>(Map<'a, A, B>, Map<'a, B, A>);

impl<'a, A, B> ReversableMap<'a, A, B> {
    pub const fn new(&self, forward_namespace: &'a str, reverse_namespace: &'a str) -> Self {
        Self(Map::new(forward_namespace), Map::new(reverse_namespace))
    }

    pub fn reverse(self) -> ReversableMap<'a, B, A> {
        ReversableMap(self.1, self.0)
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
