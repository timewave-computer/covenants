use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Uint128, Attribute};
use covenant_macros::{clocked, covenant_deposit_address, covenant_clock_address};

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
    pub expected_ls_token_amount: Uint128,
    pub allowed_return_delta: Uint128,
    pub expected_native_token_amount: Uint128,
}

#[cw_serde]
pub struct LpConfig {
    /// the native token amount we expect to be funded with
    pub expected_native_token_amount: Uint128,
    /// stride redemption rate is variable so we set the expected ls token amount 
    pub expected_ls_token_amount: Uint128,
    /// accepted return amount fluctuation that gets applied to EXPECTED_LS_TOKEN_AMOUNT
    pub allowed_return_delta: Uint128,
    /// address of the liquidity pool we plan to enter
    pub pool_address: Addr,
    /// amounts of native and ls tokens we consider ok to single-side lp
    pub single_side_lp_limits: SingleSideLpLimits,
    /// boolean flag for enabling autostaking of LP tokens upon liquidity provisioning
    pub autostake: Option<bool>,
    /// slippage tolerance parameter for liquidity provisioning 
    pub slippage_tolerance: Option<Decimal>,
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
            Attribute::new("expected_native_token_amount", self.expected_native_token_amount.to_string()),
            Attribute::new("expected_ls_token_amount", self.expected_ls_token_amount.to_string()),
            Attribute::new("allowed_return_delta", self.allowed_return_delta.to_string()),
            Attribute::new("pool_address", self.pool_address.to_string()),
            Attribute::new(
                "single_side_lp_limit_native",
                self.single_side_lp_limits.native_asset_limit.to_string()
            ),
            Attribute::new(
                "single_side_lp_limit_ls",
                self.single_side_lp_limits.ls_asset_limit.to_string()
            ),
            Attribute::new("autostake", autostake),
            Attribute::new("slippage_tolerance", slippage_tolerance),
        ]
    }

    /// validates the existing pool balances to match our initial expectations.
    /// if `PriceRangeError` is returned, it most likely means that the pool had a 
    /// significant shift in its balance ratio.
    pub fn validate_price_range(&self, pool_native_bal: Uint128, pool_ls_bal: Uint128) -> Result<(), ContractError> {
        // find the min return amount by subtracting the delta from expected amount
        let min_return_amount = self.expected_ls_token_amount
            .checked_sub(self.allowed_return_delta)?;
        // find the max return amount by adding the delta to expected amount
        let max_return_amount = self.expected_ls_token_amount
            .checked_add(self.allowed_return_delta)?;
    
        // derive allowed proportions
        let min_accepted_ratio = Decimal::from_ratio(min_return_amount, self.expected_native_token_amount);
        let max_accepted_ratio = Decimal::from_ratio(max_return_amount, self.expected_native_token_amount);
    
        // we find the proportion of the price range being validated
        let validation_ratio = Decimal::from_ratio(pool_ls_bal, pool_native_bal);
    
        // if current return to offer amount ratio falls out of [min_accepted_ratio, max_return_amount],
        // return price range error
        if validation_ratio < min_accepted_ratio || validation_ratio > max_accepted_ratio {
            return Err(ContractError::PriceRangeError {});
        }
    
        Ok(())
    }
}

/// holds the native and ls asset denoms relevant for providing liquidity.
#[cw_serde]
pub struct AssetData {
    pub native_asset_denom: String,
    pub ls_asset_denom: String,
}

impl AssetData {
    /// helper method to get astroport AssetInfo for native token
    pub fn get_native_asset_info(&self) -> AssetInfo {
        AssetInfo::NativeToken {
            denom: self.native_asset_denom.to_string(),
        }
    }

    /// helper method to get astroport AssetInfo for ls token
    pub fn get_ls_asset_info(&self) -> AssetInfo {
        AssetInfo::NativeToken {
            denom: self.ls_asset_denom.to_string(),
        }
    }

    pub fn to_asset_vec(&self, native_bal: Uint128, ls_bal: Uint128) -> Vec<Asset> {
        vec![
            Asset {
                info: self.get_native_asset_info(),
                amount: native_bal,
            },
            Asset {
                info: self.get_ls_asset_info(),
                amount: ls_bal,
            },
        ]
    }
}

/// single side lp limits define the highest amount (in `Uint128`) that
/// we consider acceptable to provide single-sided. 
/// if asset balance exceeds these limits, double-sided liquidity should be provided.
#[cw_serde]
pub struct SingleSideLpLimits {
    pub native_asset_limit: Uint128,
    pub ls_asset_limit: Uint128,
}

/// Defines fields relevant to LP module that are known prior to covenant
/// being instantiated. Use `to_instantiate_msg` implemented method to obtain
/// the `InstantiateMsg` by providing the non-deterministic fields.
#[cw_serde]
pub struct PresetLpFields {
    /// slippage tolerance for providing liquidity
    pub slippage_tolerance: Option<Decimal>,
    /// determines whether provided liquidity is automatically staked
    pub autostake: Option<bool>,
    /// denominations of native and ls assets
    pub assets: AssetData,
    /// limits (in `Uint128`) for single side liquidity provision.
    /// Defaults to 100 if none are provided.
    pub single_side_lp_limits: Option<SingleSideLpLimits>,
    /// lp contract code
    pub lp_code: u64,
    /// label for contract to be instantiated with
    pub label: String,
    /// workaround for the current lack of stride redemption rate query.
    /// we set the expected amount of ls tokens we expect to receive for
    /// the relevant half of the native tokens we have 
    pub expected_ls_token_amount: Uint128,
    /// difference (both ways) we tolerate with regards to the `expected_ls_token_amount`
    pub allowed_return_delta: Uint128,
    /// amount of native tokens we expect to receive from depositor
    pub expected_native_token_amount: Uint128,
}

impl PresetLpFields {
    /// builds an `InstantiateMsg` by taking in any fields not known on instantiation.
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        holder_address: String,
        pool_address: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            pool_address,
            clock_address,
            holder_address,
            slippage_tolerance: self.slippage_tolerance,
            autostake: self.autostake,
            assets: self.assets,
            single_side_lp_limits: self.single_side_lp_limits.unwrap_or(SingleSideLpLimits {
                native_asset_limit: Uint128::new(100),
                ls_asset_limit: Uint128::new(100),
            }),
            allowed_return_delta: self.allowed_return_delta,
            expected_ls_token_amount: self.expected_ls_token_amount,
            expected_native_token_amount: self.expected_native_token_amount,
        }
    }
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

/// keeps track of provided native and ls asset liquidity in `Uint128`.
#[cw_serde]
pub struct ProvidedLiquidityInfo {
    pub provided_amount_ls: Uint128,
    pub provided_amount_native: Uint128,
}

/// state of the LP state machine
#[cw_serde]
pub enum ContractState {
    Instantiated,
}
