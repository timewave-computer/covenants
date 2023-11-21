use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Attribute, BankMsg, BlockInfo, Coin, CosmosMsg, Decimal, Fraction, IbcMsg, IbcTimeout,
    StdError, StdResult, Timestamp, Uint128, Uint64,
};
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    sudo::msg::RequestPacketTimeoutHeight,
};

pub mod neutron_ica {
    use cosmwasm_schema::{cw_serde, QueryResponses};
    use cosmwasm_std::{Attribute, Binary, Coin, StdError, Uint128, Uint64};
    use neutron_sdk::{
        bindings::{msg::IbcFee, types::ProtobufAny},
        NeutronResult,
    };
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
                Attribute::new(
                    "ibc_transfer_timeout",
                    self.ibc_transfer_timeout.to_string(),
                ),
                Attribute::new("ica_timeout", self.ica_timeout.to_string()),
                Attribute::new("ibc_recv_fee", recv_fee),
                Attribute::new("ibc_ack_fee", ack_fee),
                Attribute::new("ibc_timeout_fee", timeout_fee),
            ]
        }

        pub fn validate(self) -> Result<RemoteChainInfo, StdError> {
            if self.ibc_fee.ack_fee.is_empty()
                || self.ibc_fee.timeout_fee.is_empty()
                || !self.ibc_fee.recv_fee.is_empty()
            {
                return Err(StdError::generic_err("invalid IbcFee".to_string()));
            }

            Ok(self)
        }
    }

    fn coin_vec_to_string(coins: &Vec<Coin>) -> String {
        let mut str = "".to_string();
        if coins.is_empty() {
            str.push_str("[]");
        } else {
            for coin in coins {
                str.push_str(&coin.to_string());
            }
        }
        str.to_string()
    }

    pub fn get_proto_coin(
        denom: String,
        amount: Uint128,
    ) -> cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
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

// splitter
#[cw_serde]
pub struct DenomSplit {
    pub denom: String,
    pub split: SplitType,
}

#[cw_serde]
pub enum SplitType {
    Custom(SplitConfig),
    // predefined splits will go here
}

impl SplitType {
    pub fn get_split_config(self) -> Result<SplitConfig, StdError> {
        match self {
            SplitType::Custom(c) => Ok(c),
        }
    }
}

#[cw_serde]
pub struct SplitConfig {
    pub receivers: Vec<Receiver>,
}

#[cw_serde]
pub struct Receiver {
    pub addr: String,
    pub share: Decimal,
}

impl SplitConfig {
    pub fn remap_receivers_to_routers(
        &self,
        receiver_a: String,
        router_a: String,
        receiver_b: String,
        router_b: String,
    ) -> Result<SplitType, StdError> {
        let receivers = self
            .receivers
            .clone()
            .into_iter()
            .map(|receiver| {
                if receiver.addr == receiver_a {
                    Receiver {
                        addr: router_a.to_string(),
                        share: receiver.share,
                    }
                } else if receiver.addr == receiver_b {
                    Receiver {
                        addr: router_b.to_string(),
                        share: receiver.share,
                    }
                } else {
                    receiver
                }
            })
            .collect();

        Ok(SplitType::Custom(SplitConfig { receivers }))
    }

    pub fn validate(self, party_a: &str, party_b: &str) -> Result<SplitConfig, StdError> {
        let mut total_share = Decimal::zero();
        let mut party_a_entry = false;
        let mut party_b_entry = false;

        for receiver in self.receivers.iter() {
            total_share += receiver.share;
            if receiver.addr == party_a {
                party_a_entry = true;
            } else if receiver.addr == party_b {
                party_b_entry = true;
            }
        }

        if total_share != Decimal::one() {
            return Err(StdError::generic_err(
                "shares must add up to 1.0".to_string(),
            ))
        }
        else if !party_a_entry {
            return Err(StdError::generic_err(
                "missing party A entry in split".to_string(),
            ))
        } else if !party_b_entry {
            return Err(StdError::generic_err(
                "missing party B entry in split".to_string(),
            ))
        }

        Ok(self)
    }

    pub fn get_transfer_messages(
        &self,
        amount: Uint128,
        denom: String,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        let mut msgs: Vec<CosmosMsg> = vec![];

        for receiver in self.receivers.iter() {
            let entitlement = amount
                .checked_multiply_ratio(receiver.share.numerator(), receiver.share.denominator())
                .map_err(|_| StdError::generic_err("failed to checked_multiply".to_string()))?;

            let amount = Coin {
                denom: denom.to_string(),
                amount: entitlement,
            };

            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: receiver.addr.to_string(),
                amount: vec![amount],
            }));
        }
        Ok(msgs)
    }

    pub fn get_response_attribute(self, denom: String) -> Attribute {
        let mut receivers = "[".to_string();
        self.receivers.iter().for_each(|receiver| {
            receivers.push('(');
            receivers.push_str(&receiver.addr);
            receivers.push(',');
            receivers.push_str(&receiver.share.to_string());
            receivers.push_str("),");
        });
        receivers.push(']');
        Attribute::new(denom, receivers)
    }
}

pub fn get_distribution_messages(
    mut balances: Vec<Coin>,
    split_configs: Box<dyn Iterator<Item = StdResult<(String, SplitConfig)>>>,
    fallback_split: Option<SplitConfig>,
) -> Result<Vec<CosmosMsg>, StdError> {
    // first we query the contract balances
    let mut distribution_messages: Vec<CosmosMsg> = vec![];

    // then we iterate over our split config and try to match the entries to available balances
    for entry in split_configs {
        let (denom, config) = entry?;

        // we try to find the index of matching coin in available balances
        let balances_index = balances.iter().position(|coin| coin.denom == denom);
        if let Some(index) = balances_index {
            // pop the relevant coin and build the transfer messages
            let coin = balances.remove(index);
            let mut transfer_messages =
                config.get_transfer_messages(coin.amount, coin.denom.to_string())?;
            distribution_messages.append(&mut transfer_messages);
        }
    }

    // by now all explicitly defined denom splits have been removed from the
    // balances vector so we can take the remaining balances and distribute
    // them according to the fallback split (if provided)
    if let Some(split) = fallback_split {
        // get the distribution messages and add them to the list
        for leftover_bal in balances {
            let mut fallback_messages =
                split.get_transfer_messages(leftover_bal.amount, leftover_bal.denom)?;
            distribution_messages.append(&mut fallback_messages);
        }
    }

    Ok(distribution_messages)
}

/// enum based configuration for asserting expiration.
/// works by asserting the current block against enum variants.
#[cw_serde]
pub enum ExpiryConfig {
    /// no expiration configured
    None,
    /// block height based expiry config
    Block(u64),
    /// timestamp based expiry config
    Time(Timestamp),
}

impl ExpiryConfig {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        match self {
            ExpiryConfig::None => vec![Attribute::new("expiry_config", "none")],
            ExpiryConfig::Block(h) => vec![Attribute::new(
                "expiry_config_expiry_block_height",
                h.to_string(),
            )],
            ExpiryConfig::Time(t) => vec![Attribute::new(
                "expiry_config_expiry_block_timestamp",
                t.to_string(),
            )],
        }
    }

    /// validates that the lockup config being stored is not already expired.
    pub fn validate(&self, block_info: &BlockInfo) -> Result<(), StdError> {
        match self {
            ExpiryConfig::None => Ok(()),
            ExpiryConfig::Block(h) => {
                if h > &block_info.height {
                    Ok(())
                } else {
                    Err(StdError::generic_err(
                        "invalid expiry config: block height must be in the future".to_string(),
                    ))
                }
            }
            ExpiryConfig::Time(t) => {
                if t.nanos() > block_info.time.nanos() {
                    Ok(())
                } else {
                    Err(StdError::generic_err(
                        "invalid expiry config: block time must be in the future".to_string(),
                    ))
                }
            }
        }
    }

    /// compares current block info with the stored expiry config.
    /// returns false if no expiry configuration is stored.
    /// otherwise, returns true if the current block is past the stored info.
    pub fn is_expired(&self, block_info: BlockInfo) -> bool {
        match self {
            // no expiration date
            ExpiryConfig::None => false,
            // if stored expiration block height is less than or equal to the current block,
            // expired
            ExpiryConfig::Block(h) => h <= &block_info.height,
            // if stored expiration timestamp is more than or equal to the current timestamp,
            // expired
            ExpiryConfig::Time(t) => t.nanos() <= block_info.time.nanos(),
        }
    }
}

#[cw_serde]
pub enum ReceiverConfig {
    /// party expects to receive funds on the same chain
    Native(Addr),
    /// party expects to receive funds on a remote chain
    Ibc(DestinationConfig),
}

impl ReceiverConfig {
    pub fn get_response_attributes(self, party: String) -> Vec<Attribute> {
        match self {
            ReceiverConfig::Native(addr) => {
                vec![Attribute::new("receiver_config_native_addr", addr)]
            }
            ReceiverConfig::Ibc(destination_config) => destination_config
                .get_response_attributes()
                .into_iter()
                .map(|mut a| {
                    a.key = party.to_string() + &a.key;
                    a
                })
                .collect(),
        }
    }
}

#[cw_serde]
pub struct CovenantParty {
    /// authorized address of the party
    pub addr: String,
    /// denom provided by the party
    pub ibc_denom: String,
    /// information about receiver address
    pub receiver_config: ReceiverConfig,
}

impl CovenantParty {
    pub fn get_refund_msg(self, amount: Uint128, block: &BlockInfo) -> CosmosMsg {
        match self.receiver_config {
            ReceiverConfig::Native(addr) => CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.to_string(),
                amount: vec![Coin {
                    denom: self.ibc_denom,
                    amount,
                }],
            }),
            ReceiverConfig::Ibc(destination_config) => CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: destination_config.destination_chain_channel_id,
                to_address: self.addr.to_string(),
                amount: Coin {
                    denom: self.ibc_denom,
                    amount,
                },
                timeout: IbcTimeout::with_timestamp(
                    block
                        .time
                        .plus_seconds(destination_config.ibc_transfer_timeout.u64()),
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

impl CovenantPartiesConfig {
    pub fn get_response_attributes(self) -> Vec<Attribute> {
        let mut attrs = vec![
            Attribute::new("party_a_address", self.party_a.addr),
            Attribute::new("party_a_ibc_denom", self.party_a.ibc_denom),
            Attribute::new("party_b_address", self.party_b.addr),
            Attribute::new("party_b_ibc_denom", self.party_b.ibc_denom),
        ];
        attrs.extend(
            self.party_a
                .receiver_config
                .get_response_attributes("party_a_".to_string()),
        );
        attrs.extend(
            self.party_b
                .receiver_config
                .get_response_attributes("party_b_".to_string()),
        );
        attrs
    }

    pub fn match_caller_party(&self, caller: String) -> Result<CovenantParty, StdError> {
        let a = self.clone().party_a;
        let b = self.clone().party_b;
        if a.addr == caller {
            Ok(a)
        } else if b.addr == caller {
            Ok(b)
        } else {
            Err(StdError::generic_err("unauthorized"))
        }
    }
}

#[cw_serde]
pub enum CovenantTerms {
    TokenSwap(SwapCovenantTerms),
}

#[cw_serde]
pub struct SwapCovenantTerms {
    pub party_a_amount: Uint128,
    pub party_b_amount: Uint128,
}

#[cw_serde]
pub struct PolCovenantTerms {
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

#[cw_serde]
pub struct DestinationConfig {
    /// channel id of the destination chain
    pub destination_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub destination_receiver_addr: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
}

impl DestinationConfig {
    pub fn get_ibc_transfer_messages_for_coins(
        &self,
        coins: Vec<Coin>,
        current_timestamp: Timestamp,
        address: String,
    ) -> Vec<CosmosMsg<NeutronMsg>> {
        let mut messages: Vec<CosmosMsg<NeutronMsg>> = vec![];

        for coin in coins {
            if coin.denom != "untrn" {
                messages.push(CosmosMsg::Custom(NeutronMsg::IbcTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: self.destination_chain_channel_id.to_string(),
                    token: coin,
                    sender: address.to_string(),
                    receiver: self.destination_receiver_addr.to_string(),
                    timeout_height: RequestPacketTimeoutHeight {
                        revision_number: None,
                        revision_height: None,
                    },
                    timeout_timestamp: current_timestamp
                        .plus_seconds(self.ibc_transfer_timeout.u64())
                        .nanos(),
                    memo: "hi".to_string(),
                    fee: IbcFee {
                        // must be empty
                        recv_fee: vec![],
                        ack_fee: vec![cosmwasm_std::Coin {
                            denom: "untrn".to_string(),
                            amount: Uint128::new(1000),
                        }],
                        timeout_fee: vec![cosmwasm_std::Coin {
                            denom: "untrn".to_string(),
                            amount: Uint128::new(1000),
                        }],
                    },
                }));
            }
        }

        messages
    }

    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new(
                "destination_chain_channel_id",
                self.destination_chain_channel_id.to_string(),
            ),
            Attribute::new(
                "destination_receiver_addr",
                self.destination_receiver_addr.to_string(),
            ),
            Attribute::new("ibc_transfer_timeout", self.ibc_transfer_timeout),
        ]
    }
}
