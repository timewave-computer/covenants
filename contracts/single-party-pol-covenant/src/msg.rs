use astroport::factory::PairType;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use covenant_utils::{CovenantParty, DestinationConfig, ReceiverConfig};
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
    pub lockup_period: Expiration,
    pub pool_address: String,
    pub ls_info: LsInfo,
    pub party_a_single_side_limit: Uint128,
    pub party_b_single_side_limit: Uint128,
    pub ls_forwarder_config: CovenantPartyConfig,
    pub holder_forwarder_config: CovenantPartyConfig,
    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
    pub pool_pair_type: PairType,
    pub native_splitter_config: NativeSplitterConfig,
}

#[cw_serde]
pub struct NativeSplitterConfig {
    pub channel_id: String,
    pub connection_id: String,
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct LsInfo {
    pub ls_denom: String,
    pub ls_denom_on_neutron: String,
    pub ls_chain_to_neutron_channel_id: String,
    pub ls_neutron_connection_id: String,
}

impl CovenantPartyConfig {
    pub fn to_receiver_config(&self) -> ReceiverConfig {
        match self {
            CovenantPartyConfig::Interchain(config) => ReceiverConfig::Ibc(DestinationConfig {
                destination_chain_channel_id: config.host_to_party_chain_channel_id.to_string(),
                destination_receiver_addr: config.party_receiver_addr.to_string(),
                ibc_transfer_timeout: config.ibc_transfer_timeout,
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
    pub native_splitter_code: u64,
    pub liquid_pooler_code: u64,
    pub liquid_staker_code: u64,
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
pub enum ExecuteMsg {
    /// Withdraw from the LPer
    Withdraw {},
    ///
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ClockAddress {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Addr)]
    IbcForwarderAddress { ty: String },
    #[returns(Addr)]
    LiquidPoolerAddress {},
    #[returns(Addr)]
    LiquidStakerAddress {},
    #[returns(Addr)]
    SplitterAddress {},
    #[returns(Addr)]
    PartyDepositAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        holder: Option<covenant_single_party_pol_holder::msg::MigrateMsg>,
        ls_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        holder_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        splitter: Option<covenant_interchain_splitter::msg::MigrateMsg>,
        liquid_pooler: Option<covenant_astroport_liquid_pooler::msg::MigrateMsg>,
        // liquid_staker: Option<covenant_astroport_liquid_pooler::msg::MigrateMsg>,
    },
}
