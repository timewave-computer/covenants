use std::collections::BTreeMap;

use astroport::factory;
use cosmwasm_std::Addr;
use covenant_utils::split::SplitConfig;

use crate::setup::suite_builder::SuiteBuilder;


pub struct NativeSplitterInstantiate {
    pub msg: covenant_native_splitter::msg::InstantiateMsg,
}

impl From<NativeSplitterInstantiate> for covenant_native_splitter::msg::InstantiateMsg {
    fn from(value: NativeSplitterInstantiate) -> Self {
        value.msg
    }
}

impl NativeSplitterInstantiate {
    pub fn new(
        clock_address: Addr,
        splits: BTreeMap::<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
    ) -> Self {
        Self {
            msg: covenant_native_splitter::msg::InstantiateMsg {
                clock_address,
                splits,
                fallback_split,
            }
        }
    }

    pub fn with_clock_address(&mut self, addr: Addr) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_splits(&mut self, splits: BTreeMap::<String, SplitConfig>) -> &mut Self {
        self.msg.splits = splits;
        self
    }

    pub fn with_fallback_split(&mut self, fallback_split: Option<SplitConfig>) -> &mut Self {
        self.msg.fallback_split = fallback_split;
        self
    }
}

impl NativeSplitterInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        clock_address: Addr,
        splits: BTreeMap::<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
    ) -> Self {
        Self::new(
            clock_address,
            splits,
            fallback_split
        )
    }
}
