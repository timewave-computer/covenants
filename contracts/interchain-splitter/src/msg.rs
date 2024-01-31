use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, StdResult, WasmMsg};
use covenant_macros::{clocked, covenant_clock_address, covenant_deposit_address};
use covenant_utils::{instantiate2_helper::Instantiate2HelperConfig, split::SplitConfig};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of the associated clock
    pub clock_address: String,
    /// list of (denom, split) configurations
    // TODO: that is an interesting looking map
    pub splits: Vec<(String, SplitType)>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitConfig>,
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

#[cw_serde]
pub struct DenomSplit {
    pub denom: String,
    pub split: SplitType,
}

pub fn remap_splits(
    splits: Vec<DenomSplit>,
    (party_a_receiver, party_a_router): (String, String),
    (party_b_receiver, party_b_router): (String, String),
) -> StdResult<Vec<(String, SplitType)>> {
    let mut remapped_splits: Vec<(String, SplitType)> = vec![];

    for denom_split in &splits {
        match &denom_split.split {
            SplitType::Custom(config) => {
                let remapped_split = config.remap_receivers_to_routers(
                    party_a_receiver.to_string(),
                    party_a_router.to_string(),
                    party_b_receiver.to_string(),
                    party_b_router.to_string(),
                )?;
                remapped_splits.push((
                    denom_split.denom.to_string(),
                    SplitType::Custom(remapped_split),
                ));
            }
        }
    }

    Ok(remapped_splits)
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
    DistributeFallback { denoms: Vec<String> },
}

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
