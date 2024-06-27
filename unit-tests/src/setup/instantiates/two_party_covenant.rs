use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128, Uint64};
use covenant_utils::{
    split::SplitConfig, NativeCovenantParty, PoolPriceConfig, SingleSideLpLimits,
};
use cw_utils::Expiration;
use valence_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use valence_covenant_two_party_pol::msg::{CovenantPartyConfig, Timeouts};

use crate::setup::{suite_builder::SuiteBuilder, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN};

#[derive(Clone)]
pub struct TwoPartyCovenantInstantiate {
    pub msg: valence_covenant_two_party_pol::msg::InstantiateMsg,
}

impl From<TwoPartyCovenantInstantiate> for valence_covenant_two_party_pol::msg::InstantiateMsg {
    fn from(value: TwoPartyCovenantInstantiate) -> Self {
        value.msg
    }
}

impl TwoPartyCovenantInstantiate {
    pub fn with_timeouts(&mut self, timeouts: Timeouts) -> &mut Self {
        self.msg.timeouts = timeouts;
        self
    }

    pub fn with_contract_codes(
        &mut self,
        contract_codes: valence_covenant_two_party_pol::msg::CovenantContractCodeIds,
    ) -> &mut Self {
        self.msg.contract_codes = contract_codes;
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

    pub fn with_ragequit_config(
        &mut self,
        ragequit_config: Option<valence_two_party_pol_holder::msg::RagequitConfig>,
    ) -> &mut Self {
        self.msg.ragequit_config = ragequit_config;
        self
    }

    pub fn with_deposit_deadline(&mut self, deposit_deadline: Expiration) -> &mut Self {
        self.msg.deposit_deadline = deposit_deadline;
        self
    }

    pub fn with_party_a_config(
        &mut self,
        party_a_config: valence_covenant_two_party_pol::msg::CovenantPartyConfig,
    ) -> &mut Self {
        self.msg.party_a_config = party_a_config;
        self
    }

    pub fn with_party_b_config(
        &mut self,
        party_b_config: valence_covenant_two_party_pol::msg::CovenantPartyConfig,
    ) -> &mut Self {
        self.msg.party_b_config = party_b_config;
        self
    }

    pub fn with_covenant_type(
        &mut self,
        covenant_type: valence_two_party_pol_holder::msg::CovenantType,
    ) -> &mut Self {
        self.msg.covenant_type = covenant_type;
        self
    }

    pub fn with_party_a_share(&mut self, party_a_share: Decimal) -> &mut Self {
        self.msg.party_a_share = party_a_share;
        self
    }

    pub fn with_party_b_share(&mut self, party_b_share: Decimal) -> &mut Self {
        self.msg.party_b_share = party_b_share;
        self
    }

    pub fn with_pool_price_config(&mut self, pool_price_config: PoolPriceConfig) -> &mut Self {
        self.msg.pool_price_config = pool_price_config;
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

    pub fn with_emergency_committee(&mut self, emergency_committee: Option<String>) -> &mut Self {
        self.msg.emergency_committee = emergency_committee;
        self
    }

    pub fn with_liquid_pooler_config(
        &mut self,
        liquid_pooler_config: valence_covenant_two_party_pol::msg::LiquidPoolerConfig,
    ) -> &mut Self {
        self.msg.liquid_pooler_config = liquid_pooler_config;
        self
    }
}

impl TwoPartyCovenantInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        party_a_addr: Addr,
        party_b_addr: Addr,
        pool_address: Addr,
    ) -> Self {
        let contract_codes = valence_covenant_two_party_pol::msg::CovenantContractCodeIds {
            ibc_forwarder_code: builder.ibc_forwarder_code_id,
            interchain_router_code: builder.interchain_router_code_id,
            holder_code: builder.two_party_holder_code_id,
            clock_code: builder.clock_code_id,
            liquid_pooler_code: builder.astro_pooler_code_id,
            native_router_code: builder.native_router_code_id,
        };

        let mut splits = BTreeMap::new();
        splits.insert(party_a_addr.to_string(), Decimal::from_str("0.5").unwrap());
        splits.insert(party_b_addr.to_string(), Decimal::from_str("0.5").unwrap());

        let split_config = SplitConfig { receivers: splits };
        let mut denom_to_split_config_map = BTreeMap::new();
        denom_to_split_config_map.insert(DENOM_ATOM_ON_NTRN.to_string(), split_config.clone());
        denom_to_split_config_map.insert(DENOM_LS_ATOM_ON_NTRN.to_string(), split_config.clone());

        Self {
            msg: valence_covenant_two_party_pol::msg::InstantiateMsg {
                label: "valence_covenant_two_party_pol".to_string(),
                timeouts: Timeouts {
                    ica_timeout: Uint64::new(100),
                    ibc_transfer_timeout: Uint64::new(100),
                },
                contract_codes,
                clock_tick_max_gas: None,
                lockup_config: Expiration::AtHeight(200000),
                ragequit_config: None,
                deposit_deadline: Expiration::AtHeight(100000),
                party_a_config: CovenantPartyConfig::Native(NativeCovenantParty {
                    party_receiver_addr: party_a_addr.to_string(),
                    native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                    addr: party_a_addr.to_string(),
                    contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                }),
                party_b_config: CovenantPartyConfig::Native(NativeCovenantParty {
                    party_receiver_addr: party_b_addr.to_string(),
                    native_denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                    addr: party_b_addr.to_string(),
                    contribution: coin(10_000, DENOM_LS_ATOM_ON_NTRN),
                }),
                covenant_type: valence_two_party_pol_holder::msg::CovenantType::Share {},
                party_a_share: Decimal::from_str("0.5").unwrap(),
                party_b_share: Decimal::from_str("0.5").unwrap(),
                pool_price_config: PoolPriceConfig {
                    expected_spot_price: Decimal::from_str("1.0").unwrap(),
                    acceptable_price_spread: Decimal::from_str("0.1").unwrap(),
                },
                splits: denom_to_split_config_map,
                fallback_split: None,
                emergency_committee: None,
                liquid_pooler_config:
                    valence_covenant_two_party_pol::msg::LiquidPoolerConfig::Astroport(
                        AstroportLiquidPoolerConfig {
                            pool_pair_type: astroport::factory::PairType::Stable {},
                            pool_address: pool_address.to_string(),
                            asset_a_denom: DENOM_ATOM_ON_NTRN.to_string(),
                            asset_b_denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                            single_side_lp_limits: SingleSideLpLimits {
                                asset_a_limit: Uint128::new(10_000),
                                asset_b_limit: Uint128::new(10_000),
                            },
                        },
                    ),
                fallback_address: None,
                operation_mode: covenant_utils::op_mode::ContractOperationModeConfig::Permissioned(vec![])
            },
        }
    }
}
