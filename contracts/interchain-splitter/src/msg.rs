use cosmwasm_schema::cw_serde;
use covenant_macros::clocked;
use cosmwasm_std::{IbcTimeout, Uint128, CosmosMsg, BankMsg, IbcMsg, Coin};

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    pub splits: Vec<(String, SplitType)>,
}


#[clocked]
#[cw_serde]
pub enum ExecuteMsg {
}

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
    pub fn validate_to_split_config(self) -> Result<SplitConfig, ContractError> {
        match self {
            SplitType::Custom(c) => c.validate(),
            SplitType::TimewaveSplit => {
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

    /*
                RefundConfig::Native(addr) => CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.to_string(),
                amount: vec![Coin {
                    denom: self.provided_denom,
                    amount,
                }],
            }),
            RefundConfig::Ibc(r_c_i) => CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: r_c_i.channel_id,
                to_address: self.addr.to_string(),
                amount: Coin {
                    denom: self.provided_denom,
                    amount,
                },
                timeout: IbcTimeout::with_timestamp(
                    block.time.plus_seconds(r_c_i.ibc_transfer_timeout.u64()),
                ),
            }),
     */
    pub fn get_transfer_messages(self, amount: Uint128, denom: String) -> Result<Vec<CosmosMsg>, ContractError> {
        let mut msgs: Vec<CosmosMsg> = vec![];

        for (receiver_type, share) in self.receivers {
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
                    timeout: receiver.ibc_timeout,
                }),
                ReceiverType::Native(receiver) => CosmosMsg::Bank(BankMsg::Send {
                    to_address: receiver.address,
                    amount: vec![amount],
                }),
            };
            msgs.push(msg);
        }
        
        Ok(msgs)
    }
}
