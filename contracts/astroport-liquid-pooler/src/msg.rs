use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Attribute, Binary, Decimal, Uint128};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub clock_address: String,
    pub holder_address: String,
    pub slippage_tolerance: Option<Decimal>,
    pub autostake: Option<bool>,
    pub assets: AssetData,
    pub single_side_lp_limits: SingleSideLpLimits,
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

/// Defines fields relevant to LP module that are known prior to covenant
/// being instantiated. Use `to_instantiate_msg` implemented method to obtain
/// the `InstantiateMsg` by providing the non-deterministic fields.
#[cw_serde]
pub struct PresetLpFields {
    /// slippage tolerance for providing liquidity
    pub slippage_tolerance: Option<Decimal>,
    /// determines whether provided liquidity is automatically staked
    pub autostake: Option<bool>,
    /// denominations of both assets
    pub assets: AssetData,
    /// limits (in `Uint128`) for single side liquidity provision.
    /// Defaults to 100 if none are provided.
    pub single_side_lp_limits: Option<SingleSideLpLimits>,
    /// lp contract code
    pub lp_code: u64,
    /// label for contract to be instantiated with
    pub label: String,
    /// address of the target liquidity pool
    pub pool_address: String,
}

impl PresetLpFields {
    /// builds an `InstantiateMsg` by taking in any fields not known on instantiation.
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        holder_address: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            pool_address: self.pool_address,
            clock_address,
            holder_address,
            slippage_tolerance: self.slippage_tolerance,
            autostake: self.autostake,
            assets: self.assets,
            single_side_lp_limits: self.single_side_lp_limits.unwrap_or(SingleSideLpLimits {
                asset_a_limit: Uint128::new(100),
                asset_b_limit: Uint128::new(100),
            }),
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
