use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128, Uint64};
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ProvideLiquidity {
        config: OutpostProvideLiquidityConfig,
    },
}

// TODO: remove duplicate from here/covenant_utils
#[cw_serde]
pub struct OutpostProvideLiquidityConfig {
    /// id of the pool we wish to provide liquidity to
    pub pool_id: Uint64,
    /// the price which we expect to provide liquidity at
    pub expected_spot_price: Decimal,
    /// acceptable delta (both ways) of the expected price
    pub acceptable_price_spread: Decimal,
    /// slippage tolerance
    pub slippage_tolerance: Decimal,
    /// limits for single-side liquidity provision
    pub asset_1_single_side_lp_limit: Uint128,
    pub asset_2_single_side_lp_limit: Uint128,
}

#[cw_serde]
pub struct JoinPoolMsgContext {
    pub sender: String,
    pub pool_denom_1: String,
    pub pool_denom_2: String,
    pub gamm_denom: String,
}

#[cw_serde]
pub enum QueryMsg {}

pub trait OsmosisPool {
    fn validate_pool_assets_length(&self) -> Result<(), ContractError>;
    fn validate_pool_asset_weights(&self) -> Result<(), ContractError>;
    fn get_pool_cw_coins(&self) -> Result<Vec<Coin>, ContractError>;
    fn get_gamm_cw_coin(&self) -> Result<Coin, ContractError>;
}

impl OsmosisPool for Pool {
    /// validate that the pool we wish to provide liquidity
    /// to is composed of two assets
    fn validate_pool_assets_length(&self) -> Result<(), ContractError> {
        match self.pool_assets.len() {
            2 => Ok(()),
            _ => Err(ContractError::OsmosisPoolError(
                "pool must have 2 assets".to_string(),
            )),
        }
    }

    /// only gamm 50:50 pools are supported (for now)
    fn validate_pool_asset_weights(&self) -> Result<(), ContractError> {
        if self.pool_assets[0].weight != self.pool_assets[1].weight {
            Err(ContractError::PoolRatioError(format!(
                "{:?}:{:?}",
                self.pool_assets[0].weight, self.pool_assets[1].weight
            )))
        } else {
            Ok(())
        }
    }

    /// collect the pool assets into cw coins
    fn get_pool_cw_coins(&self) -> Result<Vec<Coin>, ContractError> {
        let mut pool_assets: Vec<Coin> = vec![];
        for pool_asset in self.clone().pool_assets {
            match pool_asset.token {
                Some(t) => pool_assets.push(Coin {
                    denom: t.denom,
                    amount: Uint128::from_str(&t.amount)?,
                }),
                None => {
                    return Err(ContractError::OsmosisPoolError(
                        "failed to get pool token".to_string(),
                    ))
                }
            }
        }
        Ok(pool_assets)
    }

    fn get_gamm_cw_coin(&self) -> Result<Coin, ContractError> {
        match &self.total_shares {
            Some(coin) => Ok(Coin {
                denom: coin.denom.to_string(),
                amount: Uint128::from_str(&coin.amount)?,
            }),
            None => Err(ContractError::OsmosisPoolError(
                "expected Some(total_shares), found None".to_string(),
            )),
        }
    }
}
