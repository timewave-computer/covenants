use astroport::{
    asset::{Asset, AssetInfo},
    factory::PairType,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    ensure, to_json_binary, Addr, Attribute, Binary, Decimal, StdResult, Uint128, WasmMsg,
};
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_lper_withdraw,
};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, PoolPriceConfig, SingleSideLpLimits,
};

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

impl InstantiateMsg {
    pub fn to_instantiate2_msg(
        &self,
        instantiate2_helper: &Instantiate2HelperConfig,
        admin: String,
        label: String,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin),
            code_id: instantiate2_helper.code,
            label,
            msg: to_json_binary(self)?,
            funds: vec![],
            salt: instantiate2_helper.salt.clone(),
        })
    }
}

#[cw_serde]
pub struct AstroportLiquidPoolerConfig {
    pub pool_pair_type: PairType,
    pub pool_address: String,
    pub asset_a_denom: String,
    pub asset_b_denom: String,
    pub single_side_lp_limits: SingleSideLpLimits,
}

impl AstroportLiquidPoolerConfig {
    pub fn to_instantiate_msg(
        &self,
        clock_address: String,
        holder_address: String,
        pool_price_config: PoolPriceConfig,
    ) -> InstantiateMsg {
        InstantiateMsg {
            pool_address: self.pool_address.to_string(),
            clock_address,
            single_side_lp_limits: self.single_side_lp_limits.clone(),
            pool_price_config,
            pair_type: self.pool_pair_type.clone(),
            holder_address,
            slippage_tolerance: None,
            assets: AssetData {
                asset_a_denom: self.asset_a_denom.to_string(),
                asset_b_denom: self.asset_b_denom.to_string(),
            },
        }
    }
}

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl DecimalRange {
    pub fn try_from(mid: Decimal, delta: Decimal) -> Result<DecimalRange, ContractError> {
        Ok(DecimalRange {
            min: mid.checked_sub(delta)?,
            max: mid.checked_add(delta)?,
        })
    }

    pub fn is_within_range(&self, value: Decimal) -> Result<(), ContractError> {
        ensure!(
            value >= self.min && value <= self.max,
            ContractError::PriceRangeError {}
        );
        Ok(())
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
