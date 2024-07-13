use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin, StdResult, Uint64, WasmMsg};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, op_mode::ContractOperationModeConfig,
    split::SplitConfig, CovenantParty, DestinationConfig, InterchainCovenantParty,
    NativeCovenantParty, ReceiverConfig,
};
use cw_utils::Expiration;

pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    pub label: String,
    pub timeouts: Timeouts,
    pub contract_codes: SwapCovenantContractCodeIds,
    pub clock_tick_max_gas: Option<Uint64>,
    pub lockup_config: Expiration,
    pub party_a_config: CovenantPartyConfig,
    pub party_b_config: CovenantPartyConfig,
    pub splits: BTreeMap<String, SplitConfig>,
    pub fallback_split: Option<SplitConfig>,
    pub fallback_address: Option<String>,
    pub operation_mode: ContractOperationModeConfig,
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
                denom_to_pfm_map: config.denom_to_pfm_map.clone(),
            }),
            CovenantPartyConfig::Native(config) => {
                ReceiverConfig::Native(config.party_receiver_addr.to_string())
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

    pub fn get_native_denom(&self) -> String {
        match self {
            CovenantPartyConfig::Interchain(config) => config.native_denom.to_string(),
            CovenantPartyConfig::Native(config) => config.native_denom.to_string(),
        }
    }

    pub fn get_contribution(&self) -> Coin {
        match self {
            CovenantPartyConfig::Interchain(config) => config.contribution.clone(),
            CovenantPartyConfig::Native(config) => config.contribution.clone(),
        }
    }

    pub fn get_router_instantiate2_wasm_msg(
        &self,
        label: String,
        admin: String,
        op_mode_cfg: ContractOperationModeConfig,
        covenant_denoms: BTreeSet<String>,
        instantiate2_helper: Instantiate2HelperConfig,
    ) -> StdResult<WasmMsg> {
        match self {
            CovenantPartyConfig::Interchain(party) => {
                let destination_config = DestinationConfig {
                    local_to_destination_chain_channel_id: party
                        .host_to_party_chain_channel_id
                        .to_string(),
                    destination_receiver_addr: party.party_receiver_addr.to_string(),
                    ibc_transfer_timeout: party.ibc_transfer_timeout,
                    denom_to_pfm_map: party.denom_to_pfm_map.clone(),
                };
                let instantiate_msg = valence_interchain_router::msg::InstantiateMsg {
                    op_mode_cfg,
                    destination_config,
                    denoms: covenant_denoms,
                };
                Ok(instantiate_msg.to_instantiate2_msg(&instantiate2_helper, admin, label)?)
            }
            CovenantPartyConfig::Native(party) => {
                let instantiate_msg = valence_native_router::msg::InstantiateMsg {
                    op_mode_cfg,
                    receiver_address: party.party_receiver_addr.to_string(),
                    denoms: covenant_denoms,
                };
                Ok(instantiate_msg.to_instantiate2_msg(&instantiate2_helper, admin, label)?)
            }
        }
    }
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
    pub(crate) fn to_covenant_codes_config(
        &self,
        party_a_router_code: u64,
        party_b_router_code: u64,
    ) -> CovenantContractCodes {
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
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    ClockAddress {},
    #[returns(cosmwasm_std::Addr)]
    HolderAddress {},
    #[returns(cosmwasm_std::Addr)]
    SplitterAddress {},
    #[returns(cosmwasm_std::Addr)]
    InterchainRouterAddress { party: String },
    #[returns(cosmwasm_std::Addr)]
    IbcForwarderAddress { party: String },
    #[returns(cosmwasm_std::Addr)]
    PartyDepositAddress { party: String },
    #[returns(CovenantContractCodes)]
    ContractCodes {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateCovenant {
        codes: Option<CovenantContractCodes>,
        clock: Option<valence_clock::msg::MigrateMsg>,
        holder: Option<valence_swap_holder::msg::MigrateMsg>,
        splitter: Option<valence_native_splitter::msg::MigrateMsg>,
        party_a_router: Option<RouterMigrateMsg>,
        party_b_router: Option<RouterMigrateMsg>,
        party_a_forwarder: Box<Option<valence_ibc_forwarder::msg::MigrateMsg>>,
        party_b_forwarder: Box<Option<valence_ibc_forwarder::msg::MigrateMsg>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}

#[cw_serde]
pub enum RouterMigrateMsg {
    Interchain(valence_interchain_router::msg::MigrateMsg),
    Native(valence_native_router::msg::MigrateMsg),
}

#[cw_serde]
pub struct CovenantContractCodes {
    pub clock: u64,
    pub holder: u64,
    pub party_a_router: u64,
    pub party_b_router: u64,
    pub party_a_forwarder: u64,
    pub party_b_forwarder: u64,
    pub splitter: u64,
}
