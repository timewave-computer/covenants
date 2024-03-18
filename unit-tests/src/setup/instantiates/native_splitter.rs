use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::Decimal;
use covenant_utils::split::SplitConfig;

use crate::setup::{DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

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
        clock_address: String,
        splits: BTreeMap<String, SplitConfig>,
        fallback_split: Option<SplitConfig>,
    ) -> Self {
        Self {
            msg: covenant_native_splitter::msg::InstantiateMsg {
                clock_address,
                splits,
                fallback_split,
            },
        }
    }

    pub fn with_clock_address(&mut self, addr: String) -> &mut Self {
        self.msg.clock_address = addr;
        self
    }

    pub fn with_splits(&mut self, splits: BTreeMap<String, SplitConfig>) -> &mut Self {
        self.msg.splits = splits;
        self
    }

    pub fn with_fallback_split(&mut self, fallback_split: Option<SplitConfig>) -> &mut Self {
        self.msg.fallback_split = fallback_split;
        self
    }
}

impl NativeSplitterInstantiate {
    pub fn default(clock_address: String, party_a_addr: String, party_b_addr: String) -> Self {
        let mut splits = BTreeMap::new();
        splits.insert(party_a_addr, Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_addr, Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        Self {
            msg: covenant_native_splitter::msg::InstantiateMsg {
                clock_address,
                splits: denom_to_split_config_map,
                fallback_split: Some(split_config),
            },
        }
    }
}
