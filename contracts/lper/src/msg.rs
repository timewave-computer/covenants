use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128, Binary};
use covenant_clock_derive::clocked;

use crate::state::ContractState;

#[cw_serde]
pub struct InstantiateMsg {
    pub lp_position: LPInfo,
    pub clock_address: String,
    pub holder_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
}

#[cw_serde]
pub struct AssetData {
    pub native_asset_denom: String,
    pub ls_asset_denom: String,
}

impl AssetData {
    pub fn get_native_asset_info(&self) -> AssetInfo {
        AssetInfo::NativeToken {
            denom: self.native_asset_denom.to_string(),
        }
    }

    pub fn get_ls_asset_info(&self) -> AssetInfo {
        AssetInfo::NativeToken {
            denom: self.ls_asset_denom.to_string(),
        }
    }
}

#[cw_serde]
pub struct SingleSideLpLimits {
    pub native_asset_limit: Uint128,
    pub ls_asset_limit: Uint128,
}

#[cw_serde]
pub struct PresetLpFields {
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: Option<SingleSideLpLimits>,
    pub lp_code: u64,
    pub lp_position: String,
    pub label: String,
}

impl PresetLpFields {
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        holder_address: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            lp_position: LPInfo {
                addr: self.lp_position,
            },
            clock_address,
            holder_address,
            slippage_tolerance: self.slippage_tolerance,
            autostake: self.autostake,
            assets: self.assets,
            single_side_lp_limits: self.single_side_lp_limits.unwrap_or(SingleSideLpLimits {
                native_asset_limit: Uint128::new(100),
                ls_asset_limit: Uint128::new(100),
            }),
        }
    }
}

#[cw_serde]
pub struct LPInfo {
    pub addr: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LPInfo)]
    LpPosition {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Vec<Asset>)]
    Assets {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        lp_position: Option<LPInfo>,
        holder_address: Option<String>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
