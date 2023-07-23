use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Uint128};
use covenant_clock_derive::clocked;

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
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: Option<SingleSideLpLimits>,
    pub lp_code: u64,
    pub label: String,
    pub expected_ls_token_amount: Uint128,
    pub allowed_return_delta: Uint128,
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    PoolAddress {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(Vec<Asset>)]
    Assets {},
    #[returns(Uint128)]
    ExpectedLsTokenAmount {},
    #[returns(Uint128)]
    AllowedReturnDelta {},
    #[returns(Uint128)]
    ExpectedNativeTokenAmount {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        pool_address: Option<String>,
        holder_address: Option<String>,
        expected_ls_token_amount: Option<Uint128>,
        allowed_return_delta: Option<Uint128>,
        single_side_lp_limits: Option<SingleSideLpLimits>,
        slippage_tolerance: Option<Decimal>,
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
