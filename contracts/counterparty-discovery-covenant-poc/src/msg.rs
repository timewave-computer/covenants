use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
use cosmwasm_std::{coin, Addr, Decimal, Deps, StdResult, Uint128, Uint64, WasmMsg};
use counterparty_discovery_covenant_holder::msg::RagequitConfig;
use counterparty_discovery_covenant_holder::msg::TwoPartyPolCovenantParty;
use counterparty_discovery_covenant_holder::msg::{
    CovenantType, UndiscoveredTwoPartyPolCovenantCounterparty,
};
use covenant_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use covenant_osmo_liquid_pooler::msg::OsmosisLiquidPoolerConfig;
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
    pub party_b_config: UndiscoveredTwoPartyPolCovenantCounterparty,
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

    pub fn get_contribution(&self) -> Coin {
        match self {
            CovenantPartyConfig::Interchain(config) => config.contribution.clone(),
            CovenantPartyConfig::Native(config) => config.contribution.clone(),
        }
    }

    pub fn get_addr(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.addr.to_string(),
            CovenantPartyConfig::Native(config) => config.addr.to_string(),
        }
    }

    pub fn get_controller_addr(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.party_receiver_addr.to_string(),
            CovenantPartyConfig::Native(config) => config.party_receiver_addr.to_string(),
        }
    }

    pub fn get_native_denom(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.native_denom.to_string(),
            CovenantPartyConfig::Native(config) => config.native_denom.to_string(),
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
    pub holder_code: u64,
    pub clock_code: u64,
    pub liquid_pooler_code: u64,
}

#[cw_serde]
pub(crate) struct CovenantContractCodes {
    pub clock: u64,
    pub holder: u64,
    pub liquid_pooler: u64,
}

impl CovenantContractCodeIds {
    pub(crate) fn to_covenant_codes_config(&self) -> CovenantContractCodes {
        CovenantContractCodes {
            clock: self.clock_code,
            holder: self.holder_code,
            liquid_pooler: self.liquid_pooler_code,
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
    LiquidPoolerAddress {},
    #[returns(Addr)]
    PartyDepositAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateCovenant {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        holder: Option<counterparty_discovery_covenant_holder::msg::MigrateMsg>,
        liquid_pooler: Option<LiquidPoolerMigrateMsg>,
    },
}

#[cw_serde]
pub enum LiquidPoolerMigrateMsg {
    Osmosis(covenant_osmo_liquid_pooler::msg::MigrateMsg),
    Astroport(covenant_astroport_liquid_pooler::msg::MigrateMsg),
}
