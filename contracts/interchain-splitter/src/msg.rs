use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Uint128, WasmMsg, StdError, to_json_binary};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use covenant_utils::SplitConfig;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of the associated clock
    pub clock_address: String,
    /// list of (denom, split) configurations
    pub splits: Vec<(String, SplitType)>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitConfig>,
}

#[cw_serde]
pub struct PresetInterchainSplitterFields {
    /// list of (denom, split) configurations
    pub splits: Vec<DenomSplit>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitConfig>,
    /// contract label
    pub label: String,
    /// code id for the interchain splitter contract
    pub code_id: u64,
    /// receiver address of party A
    pub party_a_addr: String,
    /// receiver address of party B
    pub party_b_addr: String,
}

#[cw_serde]
pub struct DenomSplit {
    pub denom: String,
    pub split: SplitType,
}

impl PresetInterchainSplitterFields {
    /// inserts non-deterministic fields into preset config:
    /// - replaces real receiver addresses with their routers
    /// - adds clock address
    pub fn to_instantiate_msg(
        &self,
        clock_address: String,
        party_a_router: String,
        party_b_router: String,
    ) -> Result<InstantiateMsg, ContractError> {
        let mut remapped_splits: Vec<(String, SplitType)> = vec![];

        for denom_split in &self.splits {
            match &denom_split.split {
                SplitType::Custom(config) => {
                    let remapped_split = config.remap_receivers_to_routers(
                        self.party_a_addr.to_string(),
                        party_a_router.to_string(),
                        self.party_b_addr.to_string(),
                        party_b_router.to_string(),
                    )?;
                    remapped_splits.push((
                        denom_split.denom.to_string(),
                        SplitType::Custom(remapped_split),
                    ));
                }
            }
        }

        let remapped_fallback = match &self.fallback_split {
            Some(split_config) => Some(split_config.remap_receivers_to_routers(
                self.party_a_addr.to_string(),
                party_a_router,
                self.party_b_addr.to_string(),
                party_b_router,
            )?),
            None => None,
        };

        Ok(InstantiateMsg {
            clock_address,
            splits: remapped_splits,
            fallback_split: remapped_fallback,
        })
    }

    pub fn to_instantiate2_msg(
        &self, admin_addr: String, salt: Binary,
        clock_address: String,
        party_a_router: String,
        party_b_router: String,
    ) -> Result<WasmMsg, StdError> {
        let instantiate_msg = match self.to_instantiate_msg(clock_address, party_a_router, party_b_router) {
            Ok(msg) => msg,
            Err(_) => return Err(StdError::generic_err("failed to generate regular instantiation message")),
        };

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

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
pub enum SplitType {
    Custom(SplitConfig),
    // predefined splits will go here
}

impl SplitType {
    pub fn get_split_config(self) -> Result<SplitConfig, ContractError> {
        match self {
            SplitType::Custom(c) => Ok(c),
        }
    }
}

#[covenant_clock_address]
#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SplitConfig)]
    DenomSplit { denom: String },
    #[returns(Vec<(String, SplitConfig)>)]
    Splits {},
    #[returns(SplitConfig)]
    FallbackSplit {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        fallback_split: Option<SplitConfig>,
        splits: Option<Vec<(String, SplitType)>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
