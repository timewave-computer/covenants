use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use covenant_clock::msg::PresetClockFields;
use covenant_depositor::msg::PresetDepositorFields;
use covenant_holder::msg::PresetHolderFields;
use covenant_lp::msg::PresetLpFields;
use covenant_ls::msg::PresetLsFields;
use neutron_sdk::bindings::msg::IbcFee;

const NEUTRON_DENOM: &str = "untrn";

#[cw_serde]
pub struct InstantiateMsg {
    pub label: String,
    pub preset_clock_fields: PresetClockFields,
    pub preset_ls_fields: PresetLsFields,
    pub preset_depositor_fields: PresetDepositorFields,
    pub preset_lp_fields: PresetLpFields,
    pub preset_holder_fields: PresetHolderFields,
    pub pool_address: String,
    pub ibc_msg_transfer_timeout_timestamp: Option<u64>,
    pub preset_ibc_fee: PresetIbcFee,
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
