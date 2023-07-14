use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw
    /// from the contract. Anyone can instantiate the contract.
    pub withdrawer: String,
    pub lp_address: String,
}

#[cw_serde]
pub struct PresetHolderFields {
    pub withdrawer: String,
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
    Withdraw {
        quantity: Option<Vec<Coin>>,
    },
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Queries the withdrawer address
    #[returns(Addr)]
    Withdrawer {},
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
