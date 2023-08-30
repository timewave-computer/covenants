use cosmwasm_schema::{cw_serde, QueryResponses};
use covenant_macros::{clocked, covenant_clock_address};
use cosmwasm_std::{IbcTimeout, Uint128, CosmosMsg, BankMsg, IbcMsg, Coin, Addr, Attribute};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of the associated clock
    pub clock_address: String,
    /// list of (denom, split) configurations
    pub splits: Vec<(String, SplitType)>,
    /// a split for all denoms that are not covered in the
    /// regular `splits` list. If no fallback is provided,
    /// we default to the timewave protocol guild split
    pub fallback_split: Option<SplitType>,
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
    TimewaveSplit,
}

impl SplitType {
    pub fn get_split_config(self) -> Result<SplitConfig, ContractError> {
        match self {
            SplitType::Custom(c) => Ok(c),
            SplitType::TimewaveSplit => {
                // TODO: query the timewave split contract here
                Ok(SplitConfig {
                    receivers: vec![(
                        ReceiverType::Native(NativeReceiver { address: "todo".to_string() }),
                        Uint128::new(100)
                    )],
                })
            },
        }
    }
}

#[cw_serde]
pub struct SplitConfig {
    pub receivers: Vec<(ReceiverType, Uint128)>,
}

impl SplitConfig {
    pub fn validate(self) -> Result<SplitConfig, ContractError> {
        let total_share: Uint128 = self.receivers
            .iter()
            .map(|r| r.1)
            .sum();

        if total_share == Uint128::new(100) {
            Ok(self)
        } else {
            Err(ContractError::SplitMisconfig {})
        }
    }

    pub fn get_transfer_messages(self, amount: Uint128, denom: String) -> Result<Vec<CosmosMsg>, ContractError> {
        let mut msgs: Vec<CosmosMsg> = vec![];

        for (receiver_type, share) in self.receivers.into_iter() {
            let entitlement = amount.checked_multiply_ratio(
                share,
                Uint128::new(100),
            ).map_err(|_| ContractError::SplitMisconfig {})?;
    
            let amount = Coin {
                denom: denom.to_string(),
                amount: entitlement,
            };
            let msg = match receiver_type {
                ReceiverType::Interchain(receiver) => CosmosMsg::Ibc(IbcMsg::Transfer {
                    channel_id: receiver.channel_id,
                    to_address: receiver.address,
                    amount,
                    timeout: receiver.ibc_timeout.clone(),
                }),
                ReceiverType::Native(receiver) => CosmosMsg::Bank(BankMsg::Send {
                    to_address: receiver.address.to_string(),
                    amount: vec![amount],
                }),
            };
            msgs.push(msg);
        }
        Ok(msgs)
    }

    pub fn get_response_attribute(self, denom: String) -> Attribute {
        let mut receivers = "[".to_string();
        self.receivers.iter().for_each(|(ty, share)| {
            receivers.push_str("(");
            match ty {
                ReceiverType::Interchain(i) => {
                    receivers.push_str(&i.address)
                },
                ReceiverType::Native(n) => receivers.push_str(&n.address),
            };
            receivers.push_str(&share.to_string());
            receivers.push_str("),");
        });
        receivers.push_str("]");
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
#[derive(QueryResponses)]
pub enum ProtocolGuildQueryMsg {
    #[returns(SplitConfig)]
    PublicGoodsSplit {},
}