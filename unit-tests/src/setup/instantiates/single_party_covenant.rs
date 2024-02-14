use std::collections::BTreeMap;

use cosmwasm_std::{coin, Addr, Decimal, Uint128, Uint64};
use cw_utils::Expiration;

use crate::setup::{
    suite_builder::SuiteBuilder, DENOM_LS_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_STRIDE,
    NTRN_STRIDE_CHANNEL,
};

#[derive(Clone)]
pub struct SinglePartyCovenantInstantiate {
    pub msg: covenant_single_party_pol::msg::InstantiateMsg,
}

impl From<SinglePartyCovenantInstantiate> for covenant_single_party_pol::msg::InstantiateMsg {
    fn from(value: SinglePartyCovenantInstantiate) -> Self {
        value.msg
    }
}

impl SinglePartyCovenantInstantiate {
    pub fn default(
        builder: &SuiteBuilder,
        ls_forwarder_config: covenant_single_party_pol::msg::CovenantPartyConfig,
        lp_forwarder_config: covenant_single_party_pol::msg::CovenantPartyConfig,
        remote_splitter: covenant_single_party_pol::msg::RemoteChainSplitterConfig,
        covenant_party: covenant_utils::InterchainCovenantParty,
        pooler_config: covenant_single_party_pol::msg::LiquidPoolerConfig,
        pool_price_config: covenant_utils::PoolPriceConfig,
    ) -> Self {
        let contract_codes = covenant_single_party_pol::msg::CovenantContractCodeIds {
            ibc_forwarder_code: builder.ibc_forwarder_code_id,
            interchain_router_code: builder.interchain_router_code_id,
            holder_code: builder.single_party_holder_code_id,
            clock_code: builder.clock_code_id,
            remote_chain_splitter_code: builder.remote_splitter_code_id,
            liquid_pooler_code: builder.astro_pooler_code_id,
            liquid_staker_code: builder.stride_staker_code_id,
        };

        Self::new(
            "single_party_covenant".to_string(),
            covenant_single_party_pol::msg::Timeouts {
                ica_timeout: 1000_u64.into(),
                ibc_transfer_timeout: 1000_u64.into(),
            },
            covenant_single_party_pol::msg::PresetIbcFee {
                ack_fee: 100_000_u128.into(),
                timeout_fee: 100_000_u128.into(),
            },
            contract_codes,
            None,
            Expiration::AtHeight(builder.app.block_info().height + 100000),
            covenant_single_party_pol::msg::LsInfo {
                ls_denom: DENOM_LS_ATOM_ON_STRIDE.to_string(),
                ls_denom_on_neutron: DENOM_LS_ATOM_ON_NTRN.to_string(),
                ls_chain_to_neutron_channel_id: NTRN_STRIDE_CHANNEL.1.to_string(),
                ls_neutron_connection_id: "conn-1".to_string(),
            },
            ls_forwarder_config,
            lp_forwarder_config,
            pool_price_config,
            remote_splitter,
            None,
            covenant_party,
            pooler_config,
        )
    }

    pub fn get_covenant_party(
        remote_recevier: &Addr,
        local_recevier: &Addr,
        remote_denom: &str,
        local_denom: &str,
        local_to_remote_channel_id: &str,
        remote_to_local_channel_id: &str,
        amount: u128,
        denom_to_pfm_map: BTreeMap<String, covenant_utils::PacketForwardMiddlewareConfig>,
    ) -> covenant_utils::InterchainCovenantParty {
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
            denom_to_pfm_map,
        }
    }

    pub fn get_forwarder_config_interchain(
        remote_recevier: &Addr,
        local_recevier: &Addr,
        remote_denom: &str,
        local_denom: &str,
        local_to_remote_channel_id: &str,
        remote_to_local_channel_id: &str,
        amount: u128,
    ) -> covenant_single_party_pol::msg::CovenantPartyConfig {
        covenant_single_party_pol::msg::CovenantPartyConfig::Interchain(
            SinglePartyCovenantInstantiate::get_covenant_party(
                remote_recevier,
                local_recevier,
                remote_denom,
                local_denom,
                local_to_remote_channel_id,
                remote_to_local_channel_id,
                amount,
                BTreeMap::new(),
            ),
        )
    }

    pub fn get_forwarder_config_native(
        recevier: &Addr,
        denom: &str,
        amount: u128,
    ) -> covenant_single_party_pol::msg::CovenantPartyConfig {
        covenant_single_party_pol::msg::CovenantPartyConfig::Native(
            covenant_utils::NativeCovenantParty {
                party_receiver_addr: recevier.to_string(),
                native_denom: denom.to_string(),
                addr: recevier.to_string(),
                contribution: coin(amount, denom),
            },
        )
    }

    pub fn get_remote_splitter_config(
        channel_id: impl Into<String>,
        denom: impl Into<String>,
        amount: impl Into<Uint128>,
        ls_share: Decimal,
        native_share: Decimal,
    ) -> covenant_single_party_pol::msg::RemoteChainSplitterConfig {
        covenant_single_party_pol::msg::RemoteChainSplitterConfig {
            channel_id: channel_id.into(),
            connection_id: "conn-1".to_string(),
            denom: denom.into(),
            amount: amount.into(),
            ls_share,
            native_share,
        }
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
}

impl SinglePartyCovenantInstantiate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: String,
        timeouts: covenant_single_party_pol::msg::Timeouts,
        preset_ibc_fee: covenant_single_party_pol::msg::PresetIbcFee,
        contract_codes: covenant_single_party_pol::msg::CovenantContractCodeIds,
        clock_tick_max_gas: Option<Uint64>,
        lockup_period: Expiration,
        ls_info: covenant_single_party_pol::msg::LsInfo,
        ls_forwarder_config: covenant_single_party_pol::msg::CovenantPartyConfig,
        lp_forwarder_config: covenant_single_party_pol::msg::CovenantPartyConfig,
        pool_price_config: covenant_utils::PoolPriceConfig,
        remote_chain_splitter_config: covenant_single_party_pol::msg::RemoteChainSplitterConfig,
        emergency_committee: Option<String>,
        covenant_party_config: covenant_utils::InterchainCovenantParty,
        liquid_pooler_config: covenant_single_party_pol::msg::LiquidPoolerConfig,
    ) -> Self {
        Self {
            msg: covenant_single_party_pol::msg::InstantiateMsg {
                label,
                timeouts,
                preset_ibc_fee,
                contract_codes,
                clock_tick_max_gas,
                lockup_period,
                ls_info,
                ls_forwarder_config,
                lp_forwarder_config,
                pool_price_config,
                remote_chain_splitter_config,
                emergency_committee,
                covenant_party_config,
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
        self.msg.timeouts = covenant_single_party_pol::msg::Timeouts {
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
        self.msg.preset_ibc_fee = covenant_single_party_pol::msg::PresetIbcFee {
            ack_fee: ack_fee.into(),
            timeout_fee: timeout_fee.into(),
        };
        self
    }

    pub fn with_contract_codes(
        &mut self,
        codes: covenant_single_party_pol::msg::CovenantContractCodeIds,
    ) -> &mut Self {
        self.msg.contract_codes = codes;
        self
    }

    pub fn with_clock_tick_max_gas(&mut self, clock_tick_max_gas: Option<Uint64>) -> &mut Self {
        self.msg.clock_tick_max_gas = clock_tick_max_gas;
        self
    }

    pub fn with_lockup_period(&mut self, lockup_period: Expiration) -> &mut Self {
        self.msg.lockup_period = lockup_period;
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
