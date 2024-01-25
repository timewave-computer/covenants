use std::collections::{BTreeMap, BTreeSet};

use astroport::factory::PairType;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{coin, Addr, Binary, Coin, Decimal, StdResult, Uint128, Uint64, WasmMsg};
use covenant_astroport_liquid_pooler::msg::{
    AssetData, PresetAstroLiquidPoolerFields, SingleSideLpLimits,
};
use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_native_router::msg::PresetNativeRouterFields;
use covenant_osmo_liquid_pooler::msg::{
    PartyChainInfo, PartyDenomInfo, PresetOsmoLiquidPoolerFields,
};
use covenant_two_party_pol_holder::msg::{CovenantType, PresetPolParty, RagequitConfig};
use covenant_utils::{CovenantParty, DenomSplit, DestinationConfig, ReceiverConfig, SplitConfig};
use cw_utils::Expiration;
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";
pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    pub label: String,
    pub timeouts: Timeouts,
    pub preset_ibc_fee: PresetIbcFee,
    pub contract_codes: CovenantContractCodeIds,
    pub clock_tick_max_gas: Option<Uint64>,
    pub lockup_config: Expiration,
    pub party_a_config: CovenantPartyConfig,
    pub party_b_config: CovenantPartyConfig,
    pub covenant_type: CovenantType,
    pub ragequit_config: Option<RagequitConfig>,
    pub deposit_deadline: Expiration,
    pub party_a_share: Uint64,
    pub party_b_share: Uint64,
    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
    pub splits: Vec<DenomSplit>,
    pub fallback_split: Option<SplitConfig>,
    pub emergency_committee: Option<String>,
    pub liquid_pooler_config: LiquidPoolerConfig,
}

#[cw_serde]
pub enum LiquidPoolerConfig {
    Osmosis(Box<OsmosisLiquidPoolerConfig>),
    Astroport(AstroportLiquidPoolerConfig),
}

impl LiquidPoolerConfig {
    pub fn to_instantiate2_msg(
        &self,
        admin: String,
        label: String,
        code_id: u64,
        salt: Binary,
        clock_addr: String,
        holder_addr: String,
        (expected_spot_price, acceptable_price_spread): (Decimal, Decimal),
    ) -> StdResult<WasmMsg> {
        match self {
            LiquidPoolerConfig::Osmosis(config) => Ok(PresetOsmoLiquidPoolerFields {
                label,
                code_id,
                note_address: config.note_address.to_string(),
                pool_id: config.pool_id,
                osmo_ibc_timeout: config.osmo_ibc_timeout,
                party_1_chain_info: config.party_1_chain_info.clone(),
                party_2_chain_info: config.party_2_chain_info.clone(),
                osmo_to_neutron_channel_id: config.osmo_to_neutron_channel_id.to_string(),
                party_1_denom_info: config.party_1_denom_info.clone(),
                party_2_denom_info: config.party_2_denom_info.clone(),
                osmo_outpost: config.osmo_outpost.to_string(),
                lp_token_denom: config.lp_token_denom.to_string(),
                slippage_tolerance: None,
                expected_spot_price,
                acceptable_price_spread,
                funding_duration_seconds: config.funding_duration_seconds,
            }
            .to_instantiate2_msg(
                admin,
                salt,
                clock_addr.to_string(),
                holder_addr.to_string(),
            )?),
            LiquidPoolerConfig::Astroport(config) => Ok(PresetAstroLiquidPoolerFields {
                slippage_tolerance: None,
                assets: AssetData {
                    asset_a_denom: config.asset_a_denom.to_string(),
                    asset_b_denom: config.asset_b_denom.to_string(),
                },
                // TODO: remove hardcoded limits
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(10000),
                    asset_b_limit: Uint128::new(100000),
                },
                label,
                code_id,
                expected_pool_ratio: expected_spot_price,
                acceptable_pool_ratio_delta: acceptable_price_spread,
                pair_type: config.pool_pair_type.clone(),
            }
            .to_instantiate2_msg(
                admin,
                salt,
                config.pool_address.to_string(),
                clock_addr.to_string(),
                holder_addr.to_string(),
            )?),
        }
    }
}

#[cw_serde]
pub struct OsmosisLiquidPoolerConfig {
    pub note_address: String,
    pub pool_id: Uint64,
    pub osmo_ibc_timeout: Uint64,
    pub osmo_outpost: String,
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub lp_token_denom: String,
    pub osmo_to_neutron_channel_id: String,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub funding_duration_seconds: Uint64,
}

#[cw_serde]
pub struct AstroportLiquidPoolerConfig {
    pub pool_pair_type: PairType,
    pub pool_address: String,
    pub asset_a_denom: String,
    pub asset_b_denom: String,
}

impl CovenantPartyConfig {
    pub fn to_receiver_config(&self) -> ReceiverConfig {
        match self {
            CovenantPartyConfig::Interchain(config) => ReceiverConfig::Ibc(DestinationConfig {
                local_to_destination_chain_channel_id: config
                    .host_to_party_chain_channel_id
                    .to_string(),
                destination_receiver_addr: config.party_receiver_addr.to_string(),
                ibc_transfer_timeout: config.ibc_transfer_timeout,
                denom_to_pfm_map: BTreeMap::new(),
            }),
            CovenantPartyConfig::Native(config) => {
                ReceiverConfig::Native(Addr::unchecked(config.party_receiver_addr.to_string()))
            }
        }
    }

    pub fn get_final_receiver_address(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.party_receiver_addr.to_string(),
            CovenantPartyConfig::Native(config) => config.party_receiver_addr.to_string(),
        }
    }

    pub fn to_covenant_party(&self) -> CovenantParty {
        match self {
            CovenantPartyConfig::Interchain(config) => CovenantParty {
                addr: config.addr.to_string(),
                native_denom: config.native_denom.to_string(),
                receiver_config: self.to_receiver_config(),
            },
            CovenantPartyConfig::Native(config) => CovenantParty {
                addr: config.addr.to_string(),
                native_denom: config.native_denom.to_string(),
                receiver_config: self.to_receiver_config(),
            },
        }
    }

    pub fn to_preset_pol_party(&self, party_share: Uint64) -> PresetPolParty {
        match &self {
            CovenantPartyConfig::Interchain(config) => PresetPolParty {
                contribution: coin(
                    config.contribution.amount.u128(),
                    config.native_denom.to_string(),
                ),
                host_addr: config.addr.to_string(),
                controller_addr: config.party_receiver_addr.to_string(),
                allocation: Decimal::from_ratio(party_share, Uint128::new(100)),
            },
            CovenantPartyConfig::Native(config) => PresetPolParty {
                contribution: config.contribution.clone(),
                host_addr: config.addr.to_string(),
                controller_addr: config.party_receiver_addr.to_string(),
                allocation: Decimal::from_ratio(party_share, Uint128::new(100)),
            },
        }
    }

    pub fn get_native_denom(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.native_denom.to_string(),
            CovenantPartyConfig::Native(config) => config.native_denom.to_string(),
        }
    }

    pub fn get_router_code_id(&self, contract_codes: &CovenantContractCodeIds) -> u64 {
        match self {
            CovenantPartyConfig::Native(_) => contract_codes.native_router_code,
            CovenantPartyConfig::Interchain(_) => contract_codes.interchain_router_code,
        }
    }

    pub fn to_router_instantiate2_msg(
        &self,
        admin_addr: String,
        clock_addr: String,
        salt: Binary,
        code_id: u64,
        label: String,
        denoms: BTreeSet<String>,
    ) -> StdResult<WasmMsg> {
        match self {
            CovenantPartyConfig::Interchain(party) => {
                let preset_party_b_router_fields = PresetInterchainRouterFields {
                    destination_config: DestinationConfig {
                        local_to_destination_chain_channel_id: party
                            .host_to_party_chain_channel_id
                            .to_string(),
                        destination_receiver_addr: party.party_receiver_addr.to_string(),
                        ibc_transfer_timeout: party.ibc_transfer_timeout,
                        denom_to_pfm_map: BTreeMap::new(),
                    },
                    label,
                    code_id,
                    denoms,
                };
                Ok(preset_party_b_router_fields.to_instantiate2_msg(
                    admin_addr,
                    salt,
                    clock_addr.to_string(),
                )?)
            }
            CovenantPartyConfig::Native(party) => {
                let preset_native_router_fields = PresetNativeRouterFields {
                    receiver_address: party.party_receiver_addr.to_string(),
                    denoms,
                    label,
                    code_id,
                };
                Ok(preset_native_router_fields.to_instantiate2_msg(
                    admin_addr,
                    salt,
                    clock_addr.to_string(),
                )?)
            }
        }
    }
}

#[cw_serde]
pub enum CovenantPartyConfig {
    Interchain(InterchainCovenantParty),
    Native(NativeCovenantParty),
}

#[cw_serde]
pub struct NativeCovenantParty {
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// denom provided by the party on neutron
    pub native_denom: String,
    /// authorized address of the party on neutron
    pub addr: String,
    /// coin provided by the party on its native chain
    pub contribution: Coin,
}

#[cw_serde]
pub struct InterchainCovenantParty {
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// connection id to the party chain
    pub party_chain_connection_id: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
    /// channel id from party to host chain
    pub party_to_host_chain_channel_id: String,
    /// channel id from host chain to the party chain
    pub host_to_party_chain_channel_id: String,
    /// denom provided by the party on its native chain
    pub remote_chain_denom: String,
    /// authorized address of the party on neutron
    pub addr: String,
    /// denom provided by the party on neutron
    pub native_denom: String,
    /// coin provided by the party on its native chain
    pub contribution: Coin,
}

#[cw_serde]
pub struct CovenantContractCodeIds {
    pub ibc_forwarder_code: u64,
    pub holder_code: u64,
    pub clock_code: u64,
    pub interchain_router_code: u64,
    pub native_router_code: u64,
    pub liquid_pooler_code: u64,
}

#[cw_serde]
pub struct Timeouts {
    /// ica timeout in seconds
    pub ica_timeout: Uint64,
    /// ibc transfer timeout in seconds
    pub ibc_transfer_timeout: Uint64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            ica_timeout: Uint64::new(DEFAULT_TIMEOUT),
            ibc_transfer_timeout: Uint64::new(DEFAULT_TIMEOUT),
        }
    }
}

#[cw_serde]
pub struct PresetIbcFee {
    pub ack_fee: Uint128,
    pub timeout_fee: Uint128,
}

impl PresetIbcFee {
    pub fn to_ibc_fee(&self) -> IbcFee {
        IbcFee {
            // must be empty
            recv_fee: vec![],
            ack_fee: vec![cosmwasm_std::Coin {
                denom: NEUTRON_DENOM.to_string(),
                amount: self.ack_fee,
            }],
            timeout_fee: vec![cosmwasm_std::Coin {
                denom: NEUTRON_DENOM.to_string(),
                amount: self.timeout_fee,
            }],
        }
    }
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ClockAddress {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Addr)]
    IbcForwarderAddress { party: String },
    #[returns(Addr)]
    InterchainRouterAddress { party: String },
    #[returns(Addr)]
    LiquidPoolerAddress {},
    #[returns(Addr)]
    PartyDepositAddress { party: String },
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateCovenant {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        holder: Option<covenant_two_party_pol_holder::msg::MigrateMsg>,
        liquid_pooler: Option<covenant_astroport_liquid_pooler::msg::MigrateMsg>,
        party_a_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_b_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_a_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        party_b_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
    },
}
