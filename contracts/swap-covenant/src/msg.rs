use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64};
use covenant_clock::msg::PresetClockFields;
use covenant_swap_holder::msg::PresetSwapHolderFields;
use covenant_utils::SwapCovenantTerms;
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";
pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    /// contract label for this specific covenant
    pub label: String,
    /// neutron relayer fee structure
    pub preset_ibc_fee: PresetIbcFee,
    /// ibc transfer and ica timeouts passed down to relevant modules
    pub timeouts: Timeouts,

    pub ibc_forwarder_code: u64,
    pub interchain_router_code: u64,

    /// instantiation fields relevant to clock module known in advance
    pub preset_clock_fields: PresetClockFields,

    /// instantiation fields relevant to swap holder contract known in advance
    pub preset_holder_fields: PresetSwapHolderFields,
    pub covenant_terms: SwapCovenantTerms,
    pub covenant_parties: SwapCovenantParties,
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
    /// denom provided by the party
    pub provided_denom: String,
    /// channel id of the destination chain
    pub party_chain_channel_id: String,
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
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
    },
}
