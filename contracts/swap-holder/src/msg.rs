use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Attribute, Binary, StdError, WasmMsg};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use covenant_utils::{CovenantPartiesConfig, CovenantTerms, ExpiryConfig};
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address for the clock. This contract verifies
    /// that only the clock can execute Ticks
    pub clock_address: String,
    /// address of the next contract to forward the funds to.
    /// usually expected tobe the splitter.
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

#[cw_serde]
pub struct PresetSwapHolderFields {
    /// block height of covenant expiration. Position is exited
    /// automatically upon reaching that height.
    pub lockup_config: Expiration,
    /// parties engaged in the POL.
    pub parties_config: CovenantPartiesConfig,
    /// terms of the covenant
    pub covenant_terms: CovenantTerms,
    /// code id for the contract
    pub code_id: u64,
    /// contract label
    pub label: String,
}

impl PresetSwapHolderFields {
    pub fn to_instantiate_msg(
        &self,
        clock_address: String,
        next_contract: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            next_contract,
            lockup_config: self.lockup_config,
            parties_config: self.parties_config.clone(),
            covenant_terms: self.covenant_terms.clone(),
        }
    }

    pub fn to_instantiate2_msg(
        &self,
        admin_addr: String,
        salt: Binary,
        clock_address: String,
        next_contract: String,
    ) -> Result<WasmMsg, StdError> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin_addr),
            code_id: self.code_id,
            label: self.label.to_string(),
            msg: to_json_binary(&self.to_instantiate_msg(clock_address, next_contract))?,
            funds: vec![],
            salt,
        })
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
