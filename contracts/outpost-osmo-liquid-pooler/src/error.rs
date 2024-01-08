use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("only 50:50 pools are supported, got {0}")]
    PoolRatioError(String),

    #[error("Osmosis pool error: {0}")]
    OsmosisPoolError(String),

    #[error("liquidity provision error: {0}")]
    LiquidityProvisionError(String),

    #[error("Fund deposit error: expected {0} bal {1}, got {2}")]
    FundsDepositError(String, String, String),

    #[error("Slippage tolerance cannot be >= 1.0")]
    SlippageError {},

    #[error("Price range error")]
    PriceRangeError {},
}

impl ContractError {
    pub fn to_std(&self) -> StdError {
        StdError::GenericErr {
            msg: self.to_string(),
        }
    }
}
