use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, StdResult, WasmMsg};
use covenant_macros::{covenant_holder_distribute, covenant_holder_emergency_withdraw};
use covenant_utils::instantiate2_helper::Instantiate2HelperConfig;
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    /// A withdrawer is the only authorized address that can withdraw
    /// from the contract.
    pub withdrawer: String,
    /// Withdraw the funds to this address
    pub withdraw_to: String,
    /// The address that is allowed to do emergency pull out
    pub emergency_committee_addr: Option<String>,
    /// the neutron address of the liquid pooler
    pub pooler_address: String,
    /// The lockup period for the covenant
    pub lockup_period: Expiration,
}

impl InstantiateMsg {
    pub fn to_instantiate2_msg(
        &self,
        instantiate2_helper: &Instantiate2HelperConfig,
        admin: String,
        label: String,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin),
            code_id: instantiate2_helper.code,
            label,
            msg: to_json_binary(self)?,
            funds: vec![],
            salt: instantiate2_helper.salt.clone(),
        })
    }
}

#[covenant_holder_distribute]
#[covenant_holder_emergency_withdraw]
#[cw_serde]
pub enum ExecuteMsg {
    /// This is called by the withdrawer to start the withdraw process
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Queries the withdrawer address
    #[returns(cosmwasm_std::Addr)]
    Withdrawer {},
    #[returns(cosmwasm_std::Addr)]
    WithdrawTo {},
    // Queries the pooler address
    #[returns(cosmwasm_std::Addr)]
    PoolerAddress {},
    #[returns(cosmwasm_std::Addr)]
    EmergencyCommitteeAddr {},
    #[returns(Expiration)]
    LockupConfig {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        withdrawer: Option<String>,
        withdraw_to: Option<String>,
        emergency_committee: Option<String>,
        pooler_address: Option<String>,
        lockup_period: Option<Expiration>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
