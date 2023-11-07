use astroport::factory::PairType;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use covenant_two_party_pol_holder::msg::RagequitConfig;
use covenant_utils::{DenomSplit, SplitType};
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
    pub pool_address: String,
    pub ragequit_config: Option<RagequitConfig>,
    pub deposit_deadline: Expiration,
    pub party_a_share: Uint64,
    pub party_b_share: Uint64,
    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
    pub pool_pair_type: PairType,
    pub splits: Vec<DenomSplit>,
    pub fallback_split: Option<SplitType>,
}

#[cw_serde]
pub struct CovenantPartyConfig {
    /// authorized address of the party on the controller chain
    pub controller_addr: String,
    /// authorized address of the party on the host chain
    pub host_addr: String,
    /// coin provided by the party on its native chain
    pub contribution: Coin,
    /// ibc denom provided by the party on neutron
    pub ibc_denom: String,
    /// channel id from party to host chain
    pub party_to_host_chain_channel_id: String,
    /// channel id from host chain to the party chain
    pub host_to_party_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// connection id to the party chain
    pub party_chain_connection_id: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
}

#[cw_serde]
pub struct CovenantContractCodeIds {
    pub ibc_forwarder_code: u64,
    pub holder_code: u64,
    pub clock_code: u64,
    pub router_code: u64,
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
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        holder: Option<covenant_two_party_pol_holder::msg::MigrateMsg>,
        party_a_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_b_router: Option<covenant_interchain_router::msg::MigrateMsg>,
        party_a_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
        party_b_forwarder: Option<covenant_ibc_forwarder::msg::MigrateMsg>,
    },
}
