use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Attribute, Binary, Decimal, Uint128};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub clock_address: String,
    pub holder_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
}

#[cw_serde]
pub struct PresetAstroLiquidPoolerFields {
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
    pub label: String,
    pub code_id: u64,
    pub expected_pool_ratio: Decimal,
    pub acceptable_pool_ratio_delta: Decimal,
}

impl PresetAstroLiquidPoolerFields {
    pub fn to_instantiate_msg(
        &self,
        pool_address: String,
        clock_address: String,
        holder_address: String,
    ) -> InstantiateMsg {
        InstantiateMsg { 
            pool_address,
            clock_address,
            holder_address,
            slippage_tolerance: self.slippage_tolerance,
            autostake: self.autostake.clone(),
            assets: self.assets.clone(),
            single_side_lp_limits: self.single_side_lp_limits.clone(),
            expected_pool_ratio: self.expected_pool_ratio,
            acceptable_pool_ratio_delta: self.acceptable_pool_ratio_delta,
        }
    }
}

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl DecimalRange {
    pub fn new(min: Decimal, max: Decimal) -> Self {
        DecimalRange { min, max }
    }

    pub fn try_from(mid: Decimal, delta: Decimal) -> Result<DecimalRange, ContractError> {
        Ok(DecimalRange { 
            min: mid.checked_sub(delta)?,
            max: mid.checked_add(delta)?,
        })
    }

    pub fn is_within_range(&self, value: Decimal) -> Result<(), ContractError> {
        if value >= self.min && value <= self.max {
            Ok(())
        } else {
            Err(ContractError::PriceRangeError {  })
        }
    }
}

#[cw_serde]
pub struct LpConfig {
    /// address of the liquidity pool we plan to enter
    pub pool_address: Addr,
    /// amounts of both tokens we consider ok to single-side lp
    pub single_side_lp_limits: SingleSideLpLimits,
    /// boolean flag for enabling autostaking of LP tokens upon liquidity provisioning
    pub autostake: Option<bool>,
    /// slippage tolerance parameter for liquidity provisioning
    pub slippage_tolerance: Option<Decimal>,
    /// expected price range
    pub expected_pool_ratio_range: DecimalRange,
}

impl LpConfig {
    pub fn to_response_attributes(self) -> Vec<Attribute> {
        let autostake = match self.autostake {
            Some(val) => val.to_string(),
            None => "None".to_string(),
        };
        let slippage_tolerance = match self.slippage_tolerance {
            Some(val) => val.to_string(),
            None => "None".to_string(),
        };
        vec![
            Attribute::new("pool_address", self.pool_address.to_string()),
            Attribute::new(
                "single_side_asset_a_limit",
                self.single_side_lp_limits.asset_a_limit.to_string(),
            ),
            Attribute::new(
                "single_side_asset_b_limit",
                self.single_side_lp_limits.asset_b_limit.to_string(),
            ),
            Attribute::new("autostake", autostake),
            Attribute::new("slippage_tolerance", slippage_tolerance),
        ]
    }
}

/// holds the both asset denoms relevant for providing liquidity
#[cw_serde]
pub struct AssetData {
    pub asset_a_denom: String,
    pub asset_b_denom: String,
}

impl AssetData {
    pub fn to_asset_vec(&self, a_bal: Uint128, b_bal: Uint128) -> Vec<Asset> {
        vec![
            Asset {
                info: AssetInfo::NativeToken { denom: self.asset_a_denom.to_string() },
                amount: a_bal,
            },
            Asset {
                info: AssetInfo::NativeToken { denom: self.asset_b_denom.to_string() },
                amount: b_bal,
            },
        ]
    }

    /// returns tuple of (asset_A, asset_B)
    pub fn to_tuple(&self, a_bal: Uint128, b_bal: Uint128) -> (Asset, Asset) {
        (
            Asset {
                info: AssetInfo::NativeToken { denom: self.asset_a_denom.to_string() },
                amount: a_bal,
            },
            Asset {
                info: AssetInfo::NativeToken { denom: self.asset_b_denom.to_string() },
                amount: b_bal,
            },
        )
    }
}

/// single side lp limits define the highest amount (in `Uint128`) that
/// we consider acceptable to provide single-sided.
/// if asset balance exceeds these limits, double-sided liquidity should be provided.
#[cw_serde]
pub struct SingleSideLpLimits {
    pub asset_a_limit: Uint128,
    pub asset_b_limit: Uint128,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_clock_address]
#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Vec<Asset>)]
    Assets {},
    #[returns(LpConfig)]
    LpConfig {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        holder_address: Option<String>,
        assets: Option<AssetData>,
        lp_config: Option<LpConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}

/// keeps track of provided asset liquidities in `Uint128`.
#[cw_serde]
pub struct ProvidedLiquidityInfo {
    pub provided_amount_a: Uint128,
    pub provided_amount_b: Uint128,
}

/// state of the LP state machine
#[cw_serde]
pub enum ContractState {
    Instantiated,
}
