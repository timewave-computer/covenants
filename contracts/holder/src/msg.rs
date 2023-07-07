use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw
    /// from the contract. Anyone can instantiate the contract.
    pub withdrawer: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// The withdraw message can only be called by the withdrawer
    /// The withdraw can specify a quanity to be withdrawn. If no
    /// quantity is specified, the full balance is withdrawn
    Withdraw {
        quantity: Option<Vec<Coin>>,
    },
}

#[cw_serde]
pub enum QueryMsg {
    // Queries the withdrawer address
    Withdrawer {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateWithdrawer { withdrawer: String},
}
