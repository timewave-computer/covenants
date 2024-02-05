use std::collections::BTreeMap;

use cosmwasm_std::{coin, testing::mock_env, Addr, Decimal, Uint128, Uint64};
use cw_utils::Expiration;

use crate::setup::suite_builder::SuiteBuilder;

#[derive(Clone)]
pub struct SwapCovenantInstantiate {
    pub msg: covenant_swap::msg::InstantiateMsg,
}

impl From<SwapCovenantInstantiate> for covenant_swap::msg::InstantiateMsg {
    fn from(value: SwapCovenantInstantiate) -> Self {
        value.msg
    }
}

impl SwapCovenantInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        party_a_config: covenant_swap::msg::CovenantPartyConfig,
        party_b_config: covenant_swap::msg::CovenantPartyConfig,
        splits: BTreeMap<String, covenant_utils::split::SplitConfig>,
    ) -> Self {
        let contract_codes = covenant_swap::msg::SwapCovenantContractCodeIds {
            ibc_forwarder_code: builder.ibc_forwarder_code_id,
            interchain_router_code: builder.interchain_router_code_id,
            native_router_code: builder.native_router_code_id,
            splitter_code: builder.native_splitter_code_id,
            holder_code: builder.swap_holder_code_id,
            clock_code: builder.clock_code_id,
        };

        Self::new(
            "swap_covenant".to_string(),
            covenant_swap::msg::Timeouts {
                ica_timeout: 1000_u64.into(),
                ibc_transfer_timeout: 1000_u64.into(),
            },
            covenant_swap::msg::PresetIbcFee {
                ack_fee: 100_000_u128.into(),
                timeout_fee: 100_000_u128.into(),
            },
            contract_codes,
            None,
            Expiration::AtHeight(mock_env().block.height + 1000),
            party_a_config,
            party_b_config,
            splits,
            None,
        )
    }

    pub fn get_party_config_interchain(
        remote_recevier: &Addr,
        local_recevier: &Addr,
        remote_denom: &str,
        local_denom: &str,
        local_to_remote_channel_id: &str,
        remote_to_local_channel_id: &str,
        amount: u128,
    ) -> covenant_swap::msg::CovenantPartyConfig {
        covenant_swap::msg::CovenantPartyConfig::Interchain(
            covenant_utils::InterchainCovenantParty {
                party_receiver_addr: remote_recevier.to_string(),
                party_chain_connection_id: "conn-1".to_string(),
                ibc_transfer_timeout: 1000_u64.into(),
                party_to_host_chain_channel_id: remote_to_local_channel_id.to_string(),
                host_to_party_chain_channel_id: local_to_remote_channel_id.to_string(),
                remote_chain_denom: remote_denom.to_string(),
                addr: local_recevier.to_string(),
                native_denom: local_denom.to_string(),
                contribution: coin(amount, remote_denom),
                denom_to_pfm_map: BTreeMap::new(),
            },
        )
    }

    pub fn get_party_config_native(
        recevier: &Addr,
        denom: &str,
        amount: u128,
    ) -> covenant_swap::msg::CovenantPartyConfig {
        covenant_swap::msg::CovenantPartyConfig::Native(covenant_utils::NativeCovenantParty {
            party_receiver_addr: recevier.to_string(),
            native_denom: denom.to_string(),
            addr: recevier.to_string(),
            contribution: coin(amount, denom),
        })
    }

    pub fn get_split_custom(
        splits: Vec<(&str, &Vec<(&Addr, Decimal)>)>,
    ) -> BTreeMap<String, covenant_utils::split::SplitConfig> {
        let mut map = BTreeMap::new();

        splits.iter().for_each(|(denom, split)| {
            let mut receivers = BTreeMap::new();

            split.iter().for_each(|(receiver, amount)| {
                receivers.insert(receiver.to_string(), *amount);
            });

            let split = covenant_utils::split::SplitConfig { receivers };

            map.insert(denom.to_string(), split);
        });
        map
    }

    pub fn get_contract_codes() {}
}

impl SwapCovenantInstantiate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: String,
        timeouts: covenant_swap::msg::Timeouts,
        preset_ibc_fee: covenant_swap::msg::PresetIbcFee,
        contract_codes: covenant_swap::msg::SwapCovenantContractCodeIds,
        clock_tick_max_gas: Option<Uint64>,
        lockup_config: Expiration,
        party_a_config: covenant_swap::msg::CovenantPartyConfig,
        party_b_config: covenant_swap::msg::CovenantPartyConfig,
        splits: BTreeMap<String, covenant_utils::split::SplitConfig>,
        fallback_split: Option<covenant_utils::split::SplitConfig>,
    ) -> Self {
        Self {
            msg: covenant_swap::msg::InstantiateMsg {
                label,
                timeouts,
                preset_ibc_fee,
                contract_codes,
                clock_tick_max_gas,
                lockup_config,
                party_a_config,
                party_b_config,
                splits,
                fallback_split,
            },
        }
    }

    /* Change functions */
    pub fn with_label(&mut self, label: &str) -> &mut Self {
        self.msg.label = label.to_string();
        self
    }

    pub fn with_timeouts(
        &mut self,
        ica_timeout: impl Into<Uint64>,
        ibc_transfer_timeout: impl Into<Uint64>,
    ) -> &mut Self {
        self.msg.timeouts = covenant_swap::msg::Timeouts {
            ica_timeout: ica_timeout.into(),
            ibc_transfer_timeout: ibc_transfer_timeout.into(),
        };
        self
    }

    pub fn with_ibc_fee(
        &mut self,
        ack_fee: impl Into<Uint128>,
        timeout_fee: impl Into<Uint128>,
    ) -> &mut Self {
        self.msg.preset_ibc_fee = covenant_swap::msg::PresetIbcFee {
            ack_fee: ack_fee.into(),
            timeout_fee: timeout_fee.into(),
        };
        self
    }

    pub fn with_contract_codes(
        &mut self,
        codes: covenant_swap::msg::SwapCovenantContractCodeIds,
    ) -> &mut Self {
        self.msg.contract_codes = codes;
        self
    }

    pub fn with_clock_tick_max_gas(&mut self, clock_tick_max_gas: Option<Uint64>) -> &mut Self {
        self.msg.clock_tick_max_gas = clock_tick_max_gas;
        self
    }

    pub fn with_lockup_config(&mut self, lockup_config: Expiration) -> &mut Self {
        self.msg.lockup_config = lockup_config;
        self
    }

    pub fn with_party_a_config(
        &mut self,
        config: covenant_swap::msg::CovenantPartyConfig,
    ) -> &mut Self {
        self.msg.party_a_config = config;
        self
    }

    pub fn with_party_b_config(
        &mut self,
        config: covenant_swap::msg::CovenantPartyConfig,
    ) -> &mut Self {
        self.msg.party_b_config = config;
        self
    }

    pub fn with_splits(
        &mut self,
        splits: BTreeMap<String, covenant_utils::split::SplitConfig>,
    ) -> &mut Self {
        self.msg.splits = splits;
        self
    }

    pub fn with_fallback_split(&mut self, split: &[(&Addr, Decimal)]) -> &mut Self {
        let mut receivers = BTreeMap::new();
        split.iter().for_each(|(receiver, amount)| {
            receivers.insert(receiver.to_string(), *amount);
        });

        self.msg.fallback_split = Some(covenant_utils::split::SplitConfig { receivers });
        self
    }
}
