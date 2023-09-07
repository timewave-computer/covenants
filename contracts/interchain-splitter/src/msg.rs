use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, IbcTimeout, Uint128,
};
use covenant_macros::{clocked, covenant_clock_address};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of the associated clock
    pub clock_address: String,
    /// list of (denom, split) configurations
    pub splits: Vec<(String, SplitType)>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitType>,
}


#[cw_serde]
pub struct PresetInterchainSplitterFields {
    /// list of (denom, split) configurations
    pub splits: Vec<(String, SplitType)>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list
    pub fallback_split: Option<SplitType>,
    /// contract label
    pub label: String,
}

impl PresetInterchainSplitterFields {
    /// inserts non-deterministic fields into preset config:
    /// - replaces real receiver addresses with their routers
    /// - adds clock address
    pub fn to_instantiate_msg(
        self,
        clock_address: String,
        party_a_router: String,
        party_a_addr: String,
        party_b_router: String,
        party_b_addr: String,
    ) -> Result<InstantiateMsg, ContractError> {
        let mut remapped_splits: Vec<(String, SplitType)> = vec![];

        for (denom, split_type) in self.splits {
            match split_type {
                SplitType::Custom(config) => {
                    let remapped_split = config.remap_receivers_to_routers(
                        party_a_addr.to_string(),
                        party_a_router.to_string(),
                        party_b_addr.to_string(),
                        party_b_router.to_string(),
                    )?;
                    remapped_splits.push((denom, remapped_split));
                },
            }
        }

        let remapped_fallback = match self.fallback_split {
            Some(split_type) => match split_type {
                SplitType::Custom(config) => Some(config.remap_receivers_to_routers(
                    party_a_addr.to_string(),
                    party_a_router.to_string(),
                    party_b_addr.to_string(),
                    party_b_router.to_string(),
                )?)
            },
            None => None,
        };

        Ok(InstantiateMsg {
            clock_address,
            splits: remapped_splits,
            fallback_split: remapped_fallback,
        })
    }
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

// for every receiver we need a few things:
#[cw_serde]
pub struct InterchainReceiver {
    // 1. remote chain channel id
    pub channel_id: String,
    // 2. receiver address
    pub address: String,
    // 3. timeout info
    pub ibc_timeout: IbcTimeout,
}

#[cw_serde]
pub struct NativeReceiver {
    pub address: String,
}

#[cw_serde]
pub enum ReceiverType {
    Interchain(InterchainReceiver),
    Native(NativeReceiver),
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

#[cw_serde]
pub struct SplitConfig {
    pub receivers: Vec<(String, Uint128)>,
}

impl SplitConfig {
    pub fn remap_receivers_to_routers(self, receiver_a: String, router_a: String, receiver_b: String, router_b: String) -> Result<SplitType, ContractError> {
        let receivers = self.receivers.into_iter()
            .map(|(addr, share)| {
                if addr == receiver_a {
                    (router_a.to_string(), share)
                } else if addr == receiver_b {
                    (router_b.to_string(), share)
                } else {
                    (addr, share)
                }
            })
            .collect();

        Ok(SplitType::Custom(SplitConfig {
            receivers,
        }))
    }

    pub fn validate(self) -> Result<SplitConfig, ContractError> {
        let total_share: Uint128 = self.receivers.iter().map(|r| r.1).sum();

        if total_share == Uint128::new(100) {
            Ok(self)
        } else {
            Err(ContractError::SplitMisconfig {})
        }
    }

    pub fn get_transfer_messages(
        &self,
        amount: Uint128,
        denom: String,
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        let mut msgs: Vec<CosmosMsg> = vec![];

        for (receiver, share) in self.receivers.iter() {
            let entitlement = amount
                .checked_multiply_ratio(*share, Uint128::new(100))
                .map_err(|_| ContractError::SplitMisconfig {})?;

            let amount = Coin {
                denom: denom.to_string(),
                amount: entitlement,
            };

            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: receiver.to_string(),
                amount: vec![amount],
            }));
        }
        Ok(msgs)
    }

    pub fn get_response_attribute(self, denom: String) -> Attribute {
        let mut receivers = "[".to_string();
        self.receivers.iter().for_each(|(ty, share)| {
            receivers.push('(');
            receivers.push_str(&ty);
            receivers.push_str(&share.to_string());
            receivers.push_str("),");
        });
        receivers.push(']');
        Attribute::new(denom, receivers)
    }
}

#[covenant_clock_address]
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

#[covenant_clock_address]
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
#[derive(QueryResponses)]
pub enum ProtocolGuildQueryMsg {
    #[returns(SplitConfig)]
    PublicGoodsSplit {},
}