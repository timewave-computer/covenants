use std::error::Error;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, StdError, WasmMsg};
use covenant_macros::covenant_holder_distribute;
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw
    /// from the contract.
    pub withdrawer: Option<String>,
    /// Withdraw the funds to this address
    pub withdraw_to: Option<String>,
    /// the neutron address of the liquid pooler
    pub pooler_address: String,
    /// The lockup period for the covenant
    pub lockup_period: Expiration,
}

/// Preset fields are set by the user when instantiating the covenant.
/// use `to_instantiate_msg` implementation method to get `InstantiateMsg`.
#[cw_serde]
pub struct PresetHolderFields {
    pub withdrawer: Option<String>,
    pub withdraw_to: Option<String>,
    pub lockup_period: Expiration,
    pub code_id: u64,
    pub label: String,
}

impl PresetHolderFields {
    /// takes in the `pool_address` from which the funds would be withdrawn
    /// and returns an `InstantiateMsg`.
    pub fn to_instantiate_msg(&self, pooler_address: String) -> InstantiateMsg {
        InstantiateMsg {
            withdrawer: self.withdrawer.clone(),
            withdraw_to: self.withdraw_to.clone(),
            pooler_address,
            lockup_period: self.lockup_period,
        }
    }

    pub fn to_instantiate2_msg(
        &self,
        admin_addr: String,
        salt: Binary,
        pooler_address: String,
    ) -> Result<WasmMsg, StdError> {
        let instantiate_msg = self.to_instantiate_msg(pooler_address);

        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin_addr),
            code_id: self.code_id,
            label: self.label.to_string(),
            msg: to_json_binary(&instantiate_msg)?,
            funds: vec![],
            salt,
        })
    }
}

#[covenant_holder_distribute]
#[cw_serde]
pub enum ExecuteMsg {
    /// This is called by the withdrawer to start the withdraw process
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Queries the withdrawer address
    #[returns(Option<Addr>)]
    Withdrawer {},
    #[returns(Option<Addr>)]
    WithdrawTo {},
    // Queries the pooler address
    #[returns(Addr)]
    PoolerAddress {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        withdrawer: Option<String>,
        withdraw_to: Option<String>,
        pooler_address: Option<String>,
        lockup_period: Option<Expiration>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
