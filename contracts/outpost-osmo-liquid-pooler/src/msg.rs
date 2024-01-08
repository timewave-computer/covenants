use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint64, Coin, Uint128, Decimal};
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ProvideLiquidity {
        pool_id: Uint64,
        min_pool_asset_ratio: Decimal,
        max_pool_asset_ratio: Decimal,
        slippage_tolerance: Decimal,
    },
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
        if self.pool_assets.len() != 2 {
            return Err(ContractError::OsmosisPoolError("pool must have 2 assets".to_string()))
        } else {
            Ok(())
        }
    }

    /// only gamm 50:50 pools are supported (for now)
    fn validate_pool_asset_weights(&self) -> Result<(), ContractError> {
        if self.pool_assets[0].weight != self.pool_assets[1].weight {
            return Err(ContractError::PoolRatioError(
                format!("{:?}:{:?}", self.pool_assets[0].weight, self.pool_assets[1].weight)
            ))
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
                None => return Err(ContractError::OsmosisPoolError("failed to get pool token".to_string()))
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