use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64};
use covenant_clock::msg::PresetClockFields;
use covenant_depositor::msg::PresetDepositorFields;
use covenant_holder::msg::PresetHolderFields;
use covenant_lp::msg::PresetLpFields;
use covenant_ls::msg::PresetLsFields;
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";
pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    /// contract label for this specific covenant
    pub label: String,
    /// instantiation fields relevant to clock module known in advance
    pub preset_clock_fields: PresetClockFields,
    /// instantiation fields relevant to ls module known in advance
    pub preset_ls_fields: PresetLsFields,
    /// instantiation fields relevant to depositor module known in advance
    pub preset_depositor_fields: PresetDepositorFields,
    /// instantiation fields relevant to lp module known in advance
    pub preset_lp_fields: PresetLpFields,
    /// instantiation fields relevant to holder module known in advance
    pub preset_holder_fields: PresetHolderFields,
    /// address of the liquidity pool we wish to interact with
    pub pool_address: String,
    /// neutron relayer fee structure
    pub preset_ibc_fee: PresetIbcFee,
    /// ibc transfer and ica timeouts passed down to relevant modules
    pub timeouts: Timeouts,
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
    DepositorAddress {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(Addr)]
    LpAddress {},
    #[returns(Addr)]
    LsAddress {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Addr)]
    PoolAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    MigrateContracts {
        clock: Option<covenant_clock::msg::MigrateMsg>,
        depositor: Option<covenant_depositor::msg::MigrateMsg>,
        lp: Option<covenant_lp::msg::MigrateMsg>,
        ls: Option<covenant_ls::msg::MigrateMsg>,
        holder: Option<covenant_holder::msg::MigrateMsg>,
    },
}
