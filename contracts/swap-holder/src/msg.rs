use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, DepsMut, StdError, StdResult, WasmMsg,
};
use covenant_clock::helpers::dequeue_msg;
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig, CovenantPartiesConfig, CovenantTerms,
};
use cw_utils::Expiration;

use crate::state::CONTRACT_STATE;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// address of the next contract to forward the funds to.
    /// usually expected to be the splitter.
    pub next_contract: String,
    /// block height of covenant expiration. Position is exited
    /// automatically upon reaching that height.
    pub lockup_config: Expiration,
    /// parties engaged in the POL.
    pub parties_config: CovenantPartiesConfig,
    /// terms of the covenant
    pub covenant_terms: CovenantTerms,
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

impl InstantiateMsg {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        let mut attrs = vec![
            Attribute::new("clock_addr", self.clock_address),
            Attribute::new("next_contract", self.next_contract),
            Attribute::new("lockup_config", self.lockup_config.to_string()),
        ];
        attrs.extend(self.parties_config.get_response_attributes());
        attrs.extend(self.covenant_terms.get_response_attributes());
        attrs
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
    #[returns(String)]
    NextContract {},
    #[returns(Expiration)]
    LockupConfig {},
    #[returns(CovenantPartiesConfig)]
    CovenantParties {},
    #[returns(CovenantTerms)]
    CovenantTerms {},
    #[returns(ContractState)]
    ContractState {},
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    /// covenant has reached its expiration date.
    Expired,
    /// underlying funds have been withdrawn.
    Complete,
}

impl ContractState {
    pub fn complete_and_dequeue(deps: DepsMut, clock_addr: &str) -> Result<WasmMsg, StdError> {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)?;
        dequeue_msg(clock_addr)
    }
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        next_contract: Option<String>,
        lockup_config: Option<Expiration>,
        parites_config: Box<Option<CovenantPartiesConfig>>,
        covenant_terms: Option<CovenantTerms>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
