use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, StdResult, Uint128, Uint64, WasmMsg};
use covenant_interchain_router::msg::PresetInterchainRouterFields;
use covenant_interchain_splitter::msg::DenomSplit;
use covenant_native_router::msg::PresetNativeRouterFields;
use covenant_utils::{
    CovenantParty, DestinationConfig, ReceiverConfig, SplitConfig, SwapCovenantTerms,
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
    pub contract_codes: SwapCovenantContractCodeIds,
    pub clock_tick_max_gas: Option<Uint64>,
    pub lockup_config: Expiration,
    pub covenant_terms: SwapCovenantTerms,
    pub party_a_config: CovenantPartyConfig,
    pub party_b_config: CovenantPartyConfig,
    pub splits: Vec<DenomSplit>,
    pub fallback_split: Option<SplitConfig>,
}

#[cw_serde]
pub enum CovenantPartyConfig {
    Interchain(InterchainCovenantParty),
    Native(NativeCovenantParty),
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

    pub fn get_router_code_id(&self, contract_codes: &SwapCovenantContractCodeIds) -> u64 {
        match self {
            CovenantPartyConfig::Interchain(_) => contract_codes.interchain_router_code,
            CovenantPartyConfig::Native(_) => contract_codes.native_router_code,
        }
    }

    pub fn get_router_instantiate2_wasm_msg(
        &self,
        label: String,
        admin: String,
        clock_addr: String,
        covenant_denoms: BTreeSet<String>,
        code_id: u64,
        salt: Binary,
    ) -> StdResult<WasmMsg> {
        match self {
            CovenantPartyConfig::Interchain(party) => {
                let destination_config = DestinationConfig {
                    local_to_destination_chain_channel_id: party
                        .host_to_party_chain_channel_id
                        .to_string(),
                    destination_receiver_addr: party.party_receiver_addr.to_string(),
                    ibc_transfer_timeout: party.ibc_transfer_timeout,
                    denom_to_pfm_map: BTreeMap::new(),
                };
                let interchain_router_fields = PresetInterchainRouterFields {
                    destination_config,
                    label,
                    code_id,
                    denoms: covenant_denoms.clone(),
                };
                Ok(interchain_router_fields.to_instantiate2_msg(
                    admin,
                    salt,
                    clock_addr.to_string(),
                )?)
            }
            CovenantPartyConfig::Native(party) => {
                let native_router_fields = PresetNativeRouterFields {
                    receiver_address: party.party_receiver_addr.to_string(),
                    denoms: covenant_denoms.clone(),
                    label,
                    code_id,
                };
                Ok(native_router_fields.to_instantiate2_msg(admin, salt, clock_addr)?)
            }
        }
    }
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
}

#[cw_serde]
pub struct NativeCovenantParty {
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// denom provided by the party on neutron
    pub native_denom: String,
    /// authorized address of the party on neutron
    pub addr: String,
}

#[cw_serde]
pub struct SwapCovenantContractCodeIds {
    pub ibc_forwarder_code: u64,
    pub interchain_router_code: u64,
    pub native_router_code: u64,
    pub splitter_code: u64,
    pub holder_code: u64,
    pub clock_code: u64,
}

impl SwapCovenantContractCodeIds {
    pub(crate) fn to_covenant_codes_config(&self, party_a_router_code: u64, party_b_router_code: u64) -> CovenantContractCodes {
        CovenantContractCodes {
            clock: self.clock_code,
            holder: self.holder_code,
            splitter: self.splitter_code,
            party_a_router: party_a_router_code,
            party_b_router: party_b_router_code,
            party_a_forwarder: self.ibc_forwarder_code,
            party_b_forwarder: self.ibc_forwarder_code,
        }
    }
}

// TODO: this config should enable the option to have both
// ibc and native chain parties
#[cw_serde]
pub struct SwapPartyConfig {
    /// authorized address of the party
    pub addr: Addr,
    /// denom provided by the party on its native chain
    pub native_denom: String,
    /// ibc denom provided by the party on neutron
    pub ibc_denom: String,
    /// channel id from party to host chain
    pub party_to_host_chain_channel_id: String,
    /// channel id from host chain to the party chain
    pub host_to_party_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub party_receiver_addr: Addr,
    /// connection id to the party chain
    pub party_chain_connection_id: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
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
    SplitterAddress {},
    #[returns(Addr)]
    InterchainRouterAddress { party: String },
    #[returns(Addr)]
    IbcForwarderAddress { party: String },
    #[returns(Addr)]
    PartyDepositAddress { party: String },
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateCovenant {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        holder: Option<covenant_swap_holder::msg::MigrateMsg>,
        splitter: Option<covenant_interchain_splitter::msg::MigrateMsg>,
        party_a_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_b_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_a_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        party_b_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
    },
}

#[cw_serde]
pub(crate) struct CovenantContractCodes {
    pub clock: u64,
    pub holder: u64,
    pub party_a_router: u64,
    pub party_b_router: u64,
    pub party_a_forwarder: u64,
    pub party_b_forwarder: u64,
    pub splitter: u64,
}
