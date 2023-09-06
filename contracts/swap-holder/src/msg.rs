use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Attribute};
use covenant_macros::{clocked, covenant_clock_address};
use covenant_utils::{LockupConfig, CovenantPartiesConfig, CovenantTerms};

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
    pub lockup_config: LockupConfig,
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
        ];
        attrs.extend(self.parties_config.get_response_attributes());
        attrs.extend(self.covenant_terms.get_response_attributes());
        attrs.extend(self.lockup_config.get_response_attributes());
        attrs
    }
}

#[cw_serde]
pub struct PresetSwapHolderFields {
    /// block height of covenant expiration. Position is exited
    /// automatically upon reaching that height.
    pub lockup_config: LockupConfig,
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
        self,
        clock_address: String,
        next_contract: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            next_contract,
            lockup_config: self.lockup_config,
            parties_config: self.parties_config,
            covenant_terms: self.covenant_terms,
        }
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[covenant_clock_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    NextContract {},
    #[returns(LockupConfig)]
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
