use astroport::factory::PairType;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64, Decimal};
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";
pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    /// contract label for this specific covenant
    pub label: String,

    /// contract codes of contracts involved
    pub contract_codes: CovenantContractCodeIds,

    /// json formatted string meant to be used for one-click
    /// liquid staking on stride
    pub autopilot_format: String,

    pub native_asset_denom: String,
    pub ls_asset_denom: String,
    pub neutron_native_asset_denom: String,
    pub neutron_ls_asset_denom: String,
    pub amount: Uint128,
    pub clock_tick_max_gas: Option<Uint64>,

    pub neutron_stride_ibc_connection_id: String,
    pub stride_neutron_ibc_transfer_channel_id: String,

    pub remote_chain_connection_id: String,
    pub remote_chain_channel_id: String,

    /// address of the liquidity pool we wish to interact with
    pub pool_address: String,
    /// neutron relayer fee structure
    pub preset_ibc_fee: PresetIbcFee,
    /// ibc transfer and ica timeouts passed down to relevant modules
    pub timeouts: Timeouts,

    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
    pub pool_pair_type: PairType,

}

#[cw_serde]
pub struct CovenantContractCodeIds {
    pub ibc_forwarder_code: u64,
    pub holder_code: u64,
    pub clock_code: u64,
    pub router_code: u64,
    pub liquid_pooler_code: u64,
    pub liquid_staker_code: u64,
    pub remote_chain_splitter_code: u64,
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
pub enum QueryMsg {}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        liquid_staker: Option<covenant_stride_liquid_staker::msg::MigrateMsg>,
        liquid_pooler: Option<covenant_astroport_liquid_pooler::msg::MigrateMsg>,
    },
}
