use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{coin, Addr, Decimal, Deps, StdResult, Uint128, Uint64, WasmMsg};
use covenant_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use covenant_osmo_liquid_pooler::msg::OsmosisLiquidPoolerConfig;
use covenant_two_party_pol_holder::msg::{CovenantType, RagequitConfig, TwoPartyPolCovenantParty};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, split::SplitConfig, CovenantParty,
    DestinationConfig, InterchainCovenantParty, NativeCovenantParty, PoolPriceConfig,
    ReceiverConfig,
};
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
    pub pool_price_config: PoolPriceConfig,
    pub splits: BTreeMap<String, SplitConfig>,
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
        instantiate2_helper: &Instantiate2HelperConfig,
        admin: String,
        label: String,
        clock_addr: String,
        holder_addr: String,
        pool_price_config: PoolPriceConfig,
    ) -> StdResult<WasmMsg> {
        match self {
            LiquidPoolerConfig::Osmosis(config) => Ok(config
                .to_instantiate_msg(
                    clock_addr.to_string(),
                    holder_addr.to_string(),
                    pool_price_config,
                )
                .to_instantiate2_msg(instantiate2_helper, admin, label)?),
            LiquidPoolerConfig::Astroport(config) => Ok(config
                .to_instantiate_msg(
                    clock_addr.to_string(),
                    holder_addr.to_string(),
                    pool_price_config,
                )
                .to_instantiate2_msg(instantiate2_helper, admin, label)?),
        }
    }
}

impl CovenantPartyConfig {
    pub fn to_receiver_config(&self, deps: Deps) -> StdResult<ReceiverConfig> {
        match self {
            CovenantPartyConfig::Interchain(config) => Ok(ReceiverConfig::Ibc(DestinationConfig {
                local_to_destination_chain_channel_id: config
                    .host_to_party_chain_channel_id
                    .to_string(),
                destination_receiver_addr: config.party_receiver_addr.to_string(),
                ibc_transfer_timeout: config.ibc_transfer_timeout,
                denom_to_pfm_map: config.denom_to_pfm_map.clone(),
            })),
            CovenantPartyConfig::Native(config) => {
                let addr = deps.api.addr_validate(&config.party_receiver_addr)?;
                Ok(ReceiverConfig::Native(addr))
            }
        }
    }

    pub fn get_final_receiver_address(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.party_receiver_addr.to_string(),
            CovenantPartyConfig::Native(config) => config.party_receiver_addr.to_string(),
        }
    }

    pub fn to_covenant_party(&self, deps: Deps) -> StdResult<CovenantParty> {
        match self {
            CovenantPartyConfig::Interchain(config) => Ok(CovenantParty {
                addr: config.addr.to_string(),
                native_denom: config.native_denom.to_string(),
                receiver_config: self.to_receiver_config(deps)?,
            }),
            CovenantPartyConfig::Native(config) => Ok(CovenantParty {
                addr: config.addr.to_string(),
                native_denom: config.native_denom.to_string(),
                receiver_config: self.to_receiver_config(deps)?,
            }),
        }
    }

    pub fn to_two_party_pol_party(
        &self,
        party_share: Uint64,
        router: String,
    ) -> TwoPartyPolCovenantParty {
        match &self {
            CovenantPartyConfig::Interchain(config) => TwoPartyPolCovenantParty {
                contribution: coin(
                    config.contribution.amount.u128(),
                    config.native_denom.to_string(),
                ),
                host_addr: config.addr.to_string(),
                controller_addr: config.party_receiver_addr.to_string(),
                allocation: Decimal::from_ratio(party_share, Uint128::new(100)),
                router,
            },
            CovenantPartyConfig::Native(config) => TwoPartyPolCovenantParty {
                contribution: config.contribution.clone(),
                host_addr: config.addr.to_string(),
                controller_addr: config.party_receiver_addr.to_string(),
                allocation: Decimal::from_ratio(party_share, Uint128::new(100)),
                router,
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
        clock_addr: Addr,
        label: String,
        denoms: BTreeSet<String>,
        instantiate2_helper: Instantiate2HelperConfig,
    ) -> StdResult<WasmMsg> {
        match self {
            CovenantPartyConfig::Interchain(party) => {
                let instantiate_msg = covenant_interchain_router::msg::InstantiateMsg {
                    clock_address: clock_addr,
                    destination_config: DestinationConfig {
                        local_to_destination_chain_channel_id: party
                            .host_to_party_chain_channel_id
                            .to_string(),
                        destination_receiver_addr: party.party_receiver_addr.to_string(),
                        ibc_transfer_timeout: party.ibc_transfer_timeout,
                        denom_to_pfm_map: party.denom_to_pfm_map.clone(),
                    },
                    denoms,
                };
                Ok(instantiate_msg.to_instantiate2_msg(&instantiate2_helper, admin_addr, label)?)
            }
            CovenantPartyConfig::Native(party) => {
                let instantiate_msg = covenant_native_router::msg::InstantiateMsg {
                    clock_address: clock_addr.to_string(),
                    receiver_address: party.party_receiver_addr.to_string(),
                    denoms,
                };
                Ok(instantiate_msg.to_instantiate2_msg(&instantiate2_helper, admin_addr, label)?)
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
pub struct CovenantContractCodeIds {
    pub ibc_forwarder_code: u64,
    pub holder_code: u64,
    pub clock_code: u64,
    pub interchain_router_code: u64,
    pub native_router_code: u64,
    pub liquid_pooler_code: u64,
}

#[cw_serde]
pub(crate) struct CovenantContractCodes {
    pub clock: u64,
    pub holder: u64,
    pub liquid_pooler: u64,
    pub party_a_router: u64,
    pub party_b_router: u64,
    pub party_a_forwarder: u64,
    pub party_b_forwarder: u64,
}

impl CovenantContractCodeIds {
    pub(crate) fn to_covenant_codes_config(
        &self,
        party_a_router_code: u64,
        party_b_router_code: u64,
    ) -> CovenantContractCodes {
        CovenantContractCodes {
            clock: self.clock_code,
            holder: self.holder_code,
            liquid_pooler: self.liquid_pooler_code,
            party_a_router: party_a_router_code,
            party_b_router: party_b_router_code,
            party_a_forwarder: self.ibc_forwarder_code,
            party_b_forwarder: self.ibc_forwarder_code,
        }
    }
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
        liquid_pooler: Option<LiquidPoolerMigrateMsg>,
        party_a_router: Option<RouterMigrateMsg>,
        party_b_router: Option<RouterMigrateMsg>,
        party_a_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        party_b_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
    },
}

#[cw_serde]
pub enum LiquidPoolerMigrateMsg {
    Osmosis(covenant_osmo_liquid_pooler::msg::MigrateMsg),
    Astroport(covenant_astroport_liquid_pooler::msg::MigrateMsg),
}

#[cw_serde]
pub enum RouterMigrateMsg {
    Interchain(covenant_interchain_router::msg::MigrateMsg),
    Native(covenant_native_router::msg::MigrateMsg),
}
