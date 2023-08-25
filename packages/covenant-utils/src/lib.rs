use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Attribute, Timestamp, StdError, Addr, Uint128, CosmosMsg, BankMsg, Coin, IbcTimeout, IbcMsg};
use neutron_ica::RemoteChainInfo;


pub mod neutron_ica {
    use cosmwasm_schema::{cw_serde, QueryResponses};
    use cosmwasm_std::{Uint64, Binary, StdError, Attribute, Coin, Uint128};
    use neutron_sdk::{bindings::{msg::IbcFee, types::ProtobufAny}, NeutronResult};
    use prost::Message;

    #[cw_serde]
    pub struct OpenAckVersion {
        pub version: String,
        pub controller_connection_id: String,
        pub host_connection_id: String,
        pub address: String,
        pub encoding: String,
        pub tx_type: String,
    }

    /// SudoPayload is a type that stores information about a transaction that we try to execute
    /// on the host chain. This is a type introduced for our convenience.
    #[cw_serde]
    pub struct SudoPayload {
        pub message: String,
        pub port_id: String,
    }

    /// Serves for storing acknowledgement calls for interchain transactions
    #[cw_serde]
    pub enum AcknowledgementResult {
        /// Success - Got success acknowledgement in sudo with array of message item types in it
        Success(Vec<String>),
        /// Error - Got error acknowledgement in sudo with payload message in it and error details
        Error((String, String)),
        /// Timeout - Got timeout acknowledgement in sudo with payload message in it
        Timeout(String),
    }

    #[cw_serde]
    pub struct RemoteChainInfo {
        /// connection id from neutron to the remote chain on which
        /// we wish to open an ICA
        pub connection_id: String,
        pub channel_id: String,
        pub denom: String,
        pub ibc_transfer_timeout: Uint64,
        pub ica_timeout: Uint64,
        pub ibc_fee: IbcFee,
    }

    impl RemoteChainInfo {
        pub fn get_response_attributes(&self) -> Vec<Attribute> {
            let recv_fee = coin_vec_to_string(&self.ibc_fee.recv_fee);
            let ack_fee = coin_vec_to_string(&self.ibc_fee.ack_fee);
            let timeout_fee = coin_vec_to_string(&self.ibc_fee.timeout_fee);

            vec![
                Attribute::new("connection_id", &self.connection_id),
                Attribute::new("channel_id", &self.channel_id),
                Attribute::new("denom", &self.denom),
                Attribute::new("ibc_transfer_timeout", &self.ibc_transfer_timeout.to_string()),
                Attribute::new("ica_timeout", &self.ica_timeout.to_string()),
                Attribute::new("ibc_recv_fee", recv_fee),
                Attribute::new("ibc_ack_fee", ack_fee),
                Attribute::new("ibc_timeout_fee", timeout_fee),
            ]
        }

        pub fn validate(self) -> Result<RemoteChainInfo, StdError> {
            if self.ibc_fee.ack_fee.is_empty() || self.ibc_fee.timeout_fee.is_empty() || !self.ibc_fee.recv_fee.is_empty() {
                return Err(StdError::GenericErr {
                    msg: "invalid IbcFee".to_string(),
                })
            }

            Ok(self)
        }
    }

    fn coin_vec_to_string(coins: &Vec<Coin>) -> String {
        let mut str = "".to_string();
        if coins.len() == 0 {
            str.push_str(&"[]".to_string());
        } else {
            for coin in coins {
                str.push_str(&coin.to_string());
            }
        }
        str.to_string()
    }

    pub fn get_proto_coin(denom: String, amount: Uint128) -> cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
        cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
            denom,
            amount: amount.to_string(),
        }
    }

    #[cw_serde]
    #[derive(QueryResponses)]
    pub enum QueryMsg {
        /// Returns the associated remote chain information
        #[returns(Option<String>)]
        DepositAddress {},
    }

    /// helper that serializes a MsgTransfer to protobuf
    pub fn to_proto_msg_transfer(msg: impl Message) -> NeutronResult<ProtobufAny> {
        // Serialize the Transfer message
        let mut buf = Vec::new();
        buf.reserve(msg.encoded_len());
        if let Err(e) = msg.encode(&mut buf) {
            return Err(StdError::generic_err(format!("Encode error: {e}")).into());
        }

        Ok(ProtobufAny {
            type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
            value: Binary::from(buf),
        })
    }

    pub fn to_proto_msg_send(msg: impl Message) -> NeutronResult<ProtobufAny> {
        // Serialize the Send message
        let mut buf = Vec::new();
        buf.reserve(msg.encoded_len());
        if let Err(e) = msg.encode(&mut buf) {
            return Err(StdError::generic_err(format!("Encode error: {e}")).into());
        }

        Ok(ProtobufAny {
            type_url: "/cosmos.bank.v1beta1.MsgSend".to_string(),
            value: Binary::from(buf),
        })
    }

    pub fn to_proto_msg_multi_send(msg: impl Message) -> NeutronResult<ProtobufAny> {
        // Serialize the Send message
        let mut buf = Vec::new();
        buf.reserve(msg.encoded_len());
        if let Err(e) = msg.encode(&mut buf) {
            return Err(StdError::generic_err(format!("Encode error: {e}")).into());
        }

        Ok(ProtobufAny {
            type_url: "/cosmos.bank.v1beta1.MsgMultiSend".to_string(),
            value: Binary::from(buf),
        })
    }
}


/// enum based configuration of the lockup period.
#[cw_serde]
pub enum LockupConfig {
    /// no lockup configured
    None,
    /// block height based lockup config
    Block(u64),
    /// timestamp based lockup config
    Time(Timestamp),
}


impl LockupConfig {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        match self {
            LockupConfig::None => vec![
                Attribute::new("lockup_config", "none"),
            ],
            LockupConfig::Block(h) => vec![
                Attribute::new("lockup_config_expiry_block_height", h.to_string()),
            ],
            LockupConfig::Time(t) => vec![
                Attribute::new("lockup_config_expiry_block_timestamp", t.to_string()),
            ],
        }
    }

    /// validates that the lockup config being stored is not already expired.
    pub fn validate(&self, block_info: BlockInfo) -> Result<(), StdError> {
        match self {
            LockupConfig::None => Ok(()),
            LockupConfig::Block(h) => {
                if h > &block_info.height {
                    Ok(())
                } else {
                    Err(StdError::GenericErr {
                        msg: "invalid lockup config: block height must be in the future".to_string()
                    })               
                }
            },
            LockupConfig::Time(t) => {
                if t.nanos() > block_info.time.nanos() {
                    Ok(())
                } else {
                    Err(StdError::GenericErr {
                        msg: "invalid lockup config: block time must be in the future".to_string()
                    })
                }
            },
        }
    }

    /// compares current block info with the stored lockup config.
    /// returns false if no lockup configuration is stored.
    /// otherwise, returns true if the current block is past the stored info.
    pub fn is_expired(self, block_info: BlockInfo) -> bool {
        match self {
            LockupConfig::None => false, // or.. true? should not be called tho
            LockupConfig::Block(h) => h <= block_info.height,
            LockupConfig::Time(t) => t.nanos() <= block_info.time.nanos(),
        }
    }
}

#[cw_serde]
pub enum RefundConfig {
    /// party expects a refund on the same chain
    Native(Addr),
    /// party expects a refund on a remote chain
    Ibc(RemoteChainInfo),
}


#[cw_serde]
pub struct CovenantParty {
    /// authorized address of the party
    pub addr: Addr,
    /// denom provided by the party
    pub provided_denom: String,
    /// config for refunding funds in case covenant fails to complete
    pub refund_config: RefundConfig,
}

impl CovenantParty {
    pub fn get_refund_msg(self, amount: Uint128, block: &BlockInfo) -> CosmosMsg  {
        match self.refund_config {
            RefundConfig::Native(addr) => CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.to_string(),
                amount: vec![
                    Coin {
                        denom: self.provided_denom,
                        amount,
                    },
                ],
            }),
            RefundConfig::Ibc(r_c_i) => CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: r_c_i.channel_id,
                to_address: self.addr.to_string(),
                amount: Coin {
                    denom: self.provided_denom,
                    amount,
                },
                timeout: IbcTimeout::with_timestamp(
                    block.time.plus_seconds(r_c_i.ibc_transfer_timeout.u64())
                ),
            }),
        }
    }
}


#[cw_serde]
pub struct CovenantPartiesConfig {
    pub party_a: CovenantParty,
    pub party_b: CovenantParty,
}


#[cw_serde]
pub enum CovenantTerms {
    TokenSwap(SwapCovenantTerms)
}

#[cw_serde]
pub struct SwapCovenantTerms {
    pub party_a_amount: Uint128,
    pub party_b_amount: Uint128,
}

impl CovenantTerms {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        match self {
            CovenantTerms::TokenSwap(terms) => {
                let attrs = vec![
                    Attribute::new("covenant_terms", "token_swap"),
                    Attribute::new("party_a_amount", terms.party_a_amount),
                    Attribute::new("party_b_amount", terms.party_b_amount),
                ];
                attrs
            }
        }
    }
}