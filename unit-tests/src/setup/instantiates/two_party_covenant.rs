use std::collections::BTreeMap;

use cosmwasm_std::{coin, Addr, Decimal, Uint128, Uint64};
use cw_utils::Expiration;

use crate::setup::suite_builder::SuiteBuilder;

#[derive(Clone)]
pub struct TwoPartyCovenantInstantiate {
    pub msg: covenant_two_party_pol::msg::InstantiateMsg,
}

impl From<TwoPartyCovenantInstantiate> for covenant_two_party_pol::msg::InstantiateMsg {
    fn from(value: TwoPartyCovenantInstantiate) -> Self {
        value.msg
    }
}

impl TwoPartyCovenantInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        pooler_config: covenant_two_party_pol::msg::LiquidPoolerConfig,
        pool_price_config: covenant_utils::PoolPriceConfig,
        covenant_type: covenant_two_party_pol_holder::msg::CovenantType,
        ragequit_config: Option<covenant_two_party_pol_holder::msg::RagequitConfig>,
        party_a_share: impl Into<Uint64>,
        party_b_share: impl Into<Uint64>,
        party_a_config: covenant_two_party_pol::msg::CovenantPartyConfig,
        party_b_config: covenant_two_party_pol::msg::CovenantPartyConfig,
        splits: BTreeMap<String, covenant_utils::split::SplitConfig>,
        fallback_split: Option<covenant_utils::split::SplitConfig>,
    ) -> Self {
        let contract_codes = covenant_two_party_pol::msg::CovenantContractCodeIds {
            ibc_forwarder_code: builder.ibc_forwarder_code_id,
            interchain_router_code: builder.interchain_router_code_id,
            holder_code: builder.two_party_holder_code_id,
            clock_code: builder.clock_code_id,
            liquid_pooler_code: builder.astro_pooler_code_id,
            native_router_code: builder.native_router_code_id,
        };

        Self::new(
            "two_party_covenant".to_string(),
            covenant_two_party_pol::msg::Timeouts {
                ica_timeout: 1000_u64.into(),
                ibc_transfer_timeout: 1000_u64.into(),
            },
            covenant_two_party_pol::msg::PresetIbcFee {
                ack_fee: 100_000_u128.into(),
                timeout_fee: 100_000_u128.into(),
            },
            contract_codes,
            None,
            Expiration::AtHeight(builder.app.block_info().height + 100000),
            None,
            pool_price_config,
            pooler_config,
            party_a_config,
            party_b_config,
            covenant_type,
            ragequit_config,
            Expiration::AtHeight(builder.app.block_info().height + 1000),
            party_a_share,
            party_b_share,
            splits,
            fallback_split,
        )
    }

    pub fn get_ragequit_config(
        penalty: Decimal,
    ) -> Option<covenant_two_party_pol_holder::msg::RagequitConfig> {
        Some(covenant_two_party_pol_holder::msg::RagequitConfig::Enabled(
            covenant_two_party_pol_holder::msg::RagequitTerms {
                penalty,
                state: None,
            },
        ))
    }

    pub fn get_party_config_interchain(
        remote_recevier: &Addr,
        local_recevier: &Addr,
        remote_denom: &str,
        local_denom: &str,
        local_to_remote_channel_id: &str,
        remote_to_local_channel_id: &str,
        amount: u128,
    ) -> covenant_two_party_pol::msg::CovenantPartyConfig {
        covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
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
    ) -> covenant_two_party_pol::msg::CovenantPartyConfig {
        covenant_two_party_pol::msg::CovenantPartyConfig::Native(
            covenant_utils::NativeCovenantParty {
                party_receiver_addr: recevier.to_string(),
                native_denom: denom.to_string(),
                addr: recevier.to_string(),
                contribution: coin(amount, denom),
            },
        )
    }

    pub fn get_astro_pooler_config(
        denom_a: impl Into<String>,
        denom_b: impl Into<String>,
        pool_addr: &Addr,
        pool_pair_type: astroport::factory::PairType,
        single_side_lp_limits: covenant_utils::SingleSideLpLimits,
    ) -> covenant_single_party_pol::msg::LiquidPoolerConfig {
        covenant_single_party_pol::msg::LiquidPoolerConfig::Astroport(
            covenant_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig {
                pool_pair_type,
                pool_address: pool_addr.to_string(),
                asset_a_denom: denom_a.into(),
                asset_b_denom: denom_b.into(),
                single_side_lp_limits,
            },
        )
    }

    pub fn get_pool_price_config(
        expected_spot_price: Decimal,
        acceptable_price_spread: Decimal,
    ) -> covenant_utils::PoolPriceConfig {
        covenant_utils::PoolPriceConfig {
            expected_spot_price,
            acceptable_price_spread,
        }
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
}

impl TwoPartyCovenantInstantiate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: String,
        timeouts: covenant_two_party_pol::msg::Timeouts,
        preset_ibc_fee: covenant_two_party_pol::msg::PresetIbcFee,
        contract_codes: covenant_two_party_pol::msg::CovenantContractCodeIds,
        clock_tick_max_gas: Option<Uint64>,
        lockup_config: Expiration,
        emergency_committee: Option<String>,
        pool_price_config: covenant_utils::PoolPriceConfig,
        liquid_pooler_config: covenant_two_party_pol::msg::LiquidPoolerConfig,
        party_a_config: covenant_two_party_pol::msg::CovenantPartyConfig,
        party_b_config: covenant_two_party_pol::msg::CovenantPartyConfig,
        covenant_type: covenant_two_party_pol_holder::msg::CovenantType,
        ragequit_config: Option<covenant_two_party_pol_holder::msg::RagequitConfig>,
        deposit_deadline: Expiration,
        party_a_share: impl Into<Uint64>,
        party_b_share: impl Into<Uint64>,
        splits: BTreeMap<String, covenant_utils::split::SplitConfig>,
        fallback_split: Option<covenant_utils::split::SplitConfig>,
    ) -> Self {
        Self {
            msg: covenant_two_party_pol::msg::InstantiateMsg {
                label,
                timeouts,
                preset_ibc_fee,
                contract_codes,
                clock_tick_max_gas,
                lockup_config,
                party_a_config,
                party_b_config,
                covenant_type,
                ragequit_config,
                deposit_deadline,
                party_a_share: party_a_share.into(),
                party_b_share: party_b_share.into(),
                pool_price_config,
                splits,
                fallback_split,
                emergency_committee,
                liquid_pooler_config,
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
        self.msg.timeouts = covenant_two_party_pol::msg::Timeouts {
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
        self.msg.preset_ibc_fee = covenant_two_party_pol::msg::PresetIbcFee {
            ack_fee: ack_fee.into(),
            timeout_fee: timeout_fee.into(),
        };
        self
    }

    pub fn with_contract_codes(
        &mut self,
        codes: covenant_two_party_pol::msg::CovenantContractCodeIds,
    ) -> &mut Self {
        self.msg.contract_codes = codes;
        self
    }

    pub fn with_clock_tick_max_gas(&mut self, clock_tick_max_gas: Option<Uint64>) -> &mut Self {
        self.msg.clock_tick_max_gas = clock_tick_max_gas;
        self
    }

    pub fn with_lockup_period(&mut self, lockup_period: Expiration) -> &mut Self {
        self.msg.lockup_config = lockup_period;
        self
    }

    pub fn with_emergency_committee(
        &mut self,
        emergency_committee: impl Into<String>,
    ) -> &mut Self {
        self.msg.emergency_committee = Some(emergency_committee.into());
        self
    }
}
