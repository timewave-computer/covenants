use astroport::{
    asset::{Asset, AssetInfo},
    factory::PairType,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Attribute, Binary, Decimal, StdError, Uint128, WasmMsg};
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_lper_withdraw,
};
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub clock_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
    pub pool_price_config: PoolPriceConfig,
    pub pair_type: PairType,
    pub holder_address: String,
}

#[cw_serde]
pub struct PresetAstroLiquidPoolerFields {
    pub slippage_tolerance: Option<Decimal>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
    pub label: String,
    pub code_id: u64,
    pub pool_price_config: PoolPriceConfig,
    pub pair_type: PairType,
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
            slippage_tolerance: self.slippage_tolerance,
            assets: self.assets.clone(),
            single_side_lp_limits: self.single_side_lp_limits.clone(),
            pool_price_config: self.pool_price_config.clone(),
            pair_type: self.pair_type.clone(),
            holder_address,
        }
    }

    pub fn to_instantiate2_msg(
        &self,
        admin_addr: String,
        salt: Binary,
        pool_address: String,
        clock_address: String,
        holder_address: String,
    ) -> Result<WasmMsg, StdError> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin_addr),
            code_id: self.code_id,
            label: self.label.to_string(),
            msg: to_json_binary(&self.to_instantiate_msg(
                pool_address,
                clock_address,
                holder_address,
            ))?,
            funds: vec![],
            salt,
        })
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
            Err(ContractError::PriceRangeError {})
        }
    }
}

#[cw_serde]
pub struct LpConfig {
    /// address of the liquidity pool we plan to enter
    pub pool_address: Addr,
    /// denoms of both parties
    pub asset_data: AssetData,
    /// amounts of both tokens we consider ok to single-side lp
    pub single_side_lp_limits: SingleSideLpLimits,
    /// slippage tolerance parameter for liquidity provisioning
    pub slippage_tolerance: Option<Decimal>,
    /// expected price range
    pub expected_pool_ratio_range: DecimalRange,
    /// pair type specified in the covenant
    pub pair_type: PairType,
}

impl LpConfig {
    pub fn to_response_attributes(self) -> Vec<Attribute> {
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
            Attribute::new("slippage_tolerance", slippage_tolerance),
            Attribute::new("party_a_denom", self.asset_data.asset_a_denom),
            Attribute::new("party_b_denom", self.asset_data.asset_b_denom),
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
                info: AssetInfo::NativeToken {
                    denom: self.asset_a_denom.to_string(),
                },
                amount: a_bal,
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: self.asset_b_denom.to_string(),
                },
                amount: b_bal,
            },
        ]
    }

    /// returns tuple of (asset_A, asset_B)
    pub fn to_tuple(&self, a_bal: Uint128, b_bal: Uint128) -> (Asset, Asset) {
        (
            Asset {
                info: AssetInfo::NativeToken {
                    denom: self.asset_a_denom.to_string(),
                },
                amount: a_bal,
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: self.asset_b_denom.to_string(),
                },
                amount: b_bal,
            },
        )
    }
}

#[clocked]
#[covenant_lper_withdraw]
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
    #[returns(LpConfig)]
    LpConfig {},
    #[returns(ProvidedLiquidityInfo)]
    ProvidedLiquidityInfo {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        holder_address: Option<String>,
        lp_config: Option<Box<LpConfig>>,
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
