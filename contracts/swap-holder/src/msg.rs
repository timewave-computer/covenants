use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Attribute, Binary, DepsMut, StdError, StdResult, WasmMsg};
use covenant_macros::{clocked, covenant_deposit_address};
use covenant_utils::{
    instantiate2_helper::Instantiate2HelperConfig,
    op_mode::{ContractOperationMode, ContractOperationModeConfig},
    CovenantPartiesConfig, CovenantTerms,
};
use cw_utils::Expiration;

use crate::state::CONTRACT_STATE;

#[cw_serde]
pub struct InstantiateMsg {
    // Contract Operation Mode.
    // The contract operation (the Tick function mostly) can either be a permissionless
    // (aka non-privileged) operation, or a permissioned operation, that is,
    // restricted to being executed by one of the configured privileged accounts.
    pub op_mode_cfg: ContractOperationModeConfig,
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
    /// refund configuration containing party router adresses
    pub refund_config: RefundConfig,
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
            Attribute::new("op_mode", format!("{:?}", self.op_mode_cfg)),
            Attribute::new("next_contract", self.next_contract),
            Attribute::new("lockup_config", self.lockup_config.to_string()),
        ];
        attrs.extend(self.parties_config.get_response_attributes());
        attrs.extend(self.covenant_terms.get_response_attributes());
        attrs
    }
}

#[cw_serde]
pub struct RefundConfig {
    pub party_a_refund_address: String,
    pub party_b_refund_address: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

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
    #[returns(RefundConfig)]
    RefundConfig {},
    #[returns(ContractOperationMode)]
    OperationMode {},
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
    pub fn complete(deps: DepsMut) -> Result<(), StdError> {
        CONTRACT_STATE.save(deps.storage, &ContractState::Complete)
    }
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        op_mode: Option<ContractOperationModeConfig>,
        next_contract: Option<String>,
        lockup_config: Option<Expiration>,
        parties_config: Box<Option<CovenantPartiesConfig>>,
        covenant_terms: Option<CovenantTerms>,
        refund_config: Option<RefundConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
