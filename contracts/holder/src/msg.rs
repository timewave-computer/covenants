use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::Coin;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw 
    /// from the contract. Anyone can instantiate the contract.
    pub withdrawer: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// The withdraw message can only be called by the withdrawer
    /// The withdraw can specify a quanity to be withdrawn. If no 
    /// quantity is specified, the full balance is withdrawn
    Withdraw {
        quantity: Option<Vec<Coin>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Queries the withdrawer address
    Withdrawer {},
}