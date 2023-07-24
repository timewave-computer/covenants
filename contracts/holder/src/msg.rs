use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw
    /// from the contract. Anyone can instantiate the contract.
    pub withdrawer: Option<String>,
    /// lp address is the address of the pool where liquidity has been provided
    /// The holder holds LP tokens associated with this pool
    pub lp_address: String,
}

// Preset fields are set by the user when instantiating the covenant
#[cw_serde]
pub struct PresetHolderFields {
    pub withdrawer: Option<String>,
    pub holder_code: u64,
    pub label: String,
}

impl PresetHolderFields {
    pub fn to_instantiate_msg(self, lp_address: String) -> InstantiateMsg {
        InstantiateMsg {
            withdrawer: self.withdrawer,
            lp_address,
        }
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    /// The withdraw message can only be called by the withdrawer
    /// The withdraw can specify a quanity to be withdrawn. If no
    /// quantity is specified, the full balance is withdrawn
    /// into withdrawer account
    Withdraw {
        quantity: Option<Vec<Coin>>,
    },
    /// The WithdrawLiqudity message can only be called by the withdrawer
    /// When it is called, the LP tokens are burned and the liquity is withdrawn
    /// from the pool and lands in the holder
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Queries the withdrawer address
    #[returns(Option<Addr>)]
    Withdrawer {},
    // Queries the pool address
    #[returns(Addr)]
    LpAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        withdrawer: Option<String>,
        lp_address: Option<String>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
