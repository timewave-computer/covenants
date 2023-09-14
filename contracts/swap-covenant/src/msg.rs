use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64};
use covenant_clock::msg::PresetClockFields;
use covenant_interchain_splitter::msg::PresetInterchainSplitterFields;
use covenant_swap_holder::msg::PresetSwapHolderFields;
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";
pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    /// contract label for this specific covenant
    pub label: String,

    /// ibc transfer and ica timeouts passed down to relevant modules
    pub timeouts: Timeouts,
    pub preset_ibc_fee: PresetIbcFee,

    pub ibc_forwarder_code: u64,
    pub interchain_router_code: u64,
    pub splitter_code: u64,

    /// instantiation fields relevant to clock module known in advance
    pub preset_clock_fields: PresetClockFields,

    /// instantiation fields relevant to swap holder contract known in advance
    pub preset_holder_fields: PresetSwapHolderFields,
    pub covenant_parties: SwapCovenantParties,
    pub preset_splitter_fields: PresetInterchainSplitterFields,
}

#[cw_serde]
pub struct SwapCovenantParties {
    pub party_a: SwapPartyConfig,
    pub party_b: SwapPartyConfig,
}

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
    pub fn to_ibc_fee(self) -> IbcFee {
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
    #[returns(SwapCovenantParties)]
    CovenantParties {},
    #[returns(Addr)]
    InterchainRouterAddress { party: String },
    #[returns(Addr)]
    IbcForwarderAddress { party: String },
    #[returns(IbcFee)]
    IbcFee {},
    #[returns(Timeouts)]
    Timeouts {},
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
    },
}
