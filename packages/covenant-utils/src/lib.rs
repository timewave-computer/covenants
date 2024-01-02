use astroport::asset::PairInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Attribute, BankMsg, BlockInfo, Coin, CosmosMsg, Decimal, Fraction, IbcMsg, IbcTimeout,
    QuerierWrapper, StdError, Timestamp, Uint128, Uint64,
};
use cw20::BalanceResponse;
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    sudo::msg::RequestPacketTimeoutHeight,
};
use std::collections::BTreeMap;

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
        let mut buf = Vec::with_capacity(msg.encoded_len());
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
        let mut buf = Vec::with_capacity(msg.encoded_len());
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
        let mut buf = Vec::with_capacity(msg.encoded_len());
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
    /// map receiver address to its share of the split
    pub receivers: BTreeMap<String, Decimal>,
}

impl SplitConfig {
    pub fn remap_receivers_to_routers(
        &self,
        receiver_a: String,
        router_a: String,
        receiver_b: String,
        router_b: String,
    ) -> Result<SplitConfig, StdError> {
        let mut new_receivers = BTreeMap::new();

        match self.receivers.get(&receiver_a) {
            Some(val) => new_receivers.insert(router_a, *val),
            None => {
                return Err(StdError::NotFound {
                    kind: format!("receiver {receiver_b:?} not found"),
                })
            }
        };
        match self.receivers.get(&receiver_b) {
            Some(val) => new_receivers.insert(router_b, *val),
            None => {
                return Err(StdError::NotFound {
                    kind: format!("receiver {receiver_b:?} not found"),
                })
            }
        };

        Ok(SplitConfig {
            receivers: new_receivers,
        })
    }

    pub fn validate(&self, party_a: &str, party_b: &str) -> Result<(), StdError> {
        let share_a = match self.receivers.get(party_a) {
            Some(val) => *val,
            None => return Err(StdError::not_found(party_a)),
        };
        let share_b = match self.receivers.get(party_b) {
            Some(val) => *val,
            None => return Err(StdError::not_found(party_b)),
        };

        if share_a + share_b != Decimal::one() {
            return Err(StdError::generic_err(
                "shares must add up to 1.0".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_transfer_messages(
        &self,
        amount: Uint128,
        denom: String,
        filter_addr: Option<String>,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        let msgs: Result<Vec<CosmosMsg>, StdError> = self
            .receivers
            .iter()
            .map(|(addr, share)| {
                // if we are filtering for a single receiver,
                // then we wish to transfer only to that receiver.
                // we thus set receiver share to 1.0, as the
                // entitlement already takes that into account.
                match &filter_addr {
                    Some(filter) => {
                        if filter == addr {
                            (addr, Decimal::one())
                        } else {
                            (addr, Decimal::zero())
                        }
                    }
                    None => (addr, *share),
                }
            })
            .filter(|(_, share)| !share.is_zero())
            .map(|(addr, share)| {
                let entitlement = amount
                    .checked_multiply_ratio(share.numerator(), share.denominator())
                    .map_err(|_| StdError::generic_err("failed to checked_multiply".to_string()))?;

                let amount = Coin {
                    denom: denom.to_string(),
                    amount: entitlement,
                };

                Ok(CosmosMsg::Bank(BankMsg::Send {
                    to_address: addr.to_string(),
                    amount: vec![amount],
                }))
            })
            .collect();

        msgs
    }

    pub fn get_response_attribute(&self, denom: String) -> Attribute {
        let mut receivers = "[".to_string();
        self.receivers.iter().for_each(|(receiver, share)| {
            receivers.push('(');
            receivers.push_str(receiver);
            receivers.push(':');
            receivers.push_str(&share.to_string());
            receivers.push_str("),");
        });
        receivers.push(']');
        Attribute::new(denom, receivers)
    }
}

// /// enum based configuration for asserting expiration.
// /// works by asserting the current block against enum variants.
// #[cw_serde]
// pub enum ExpiryConfig {
//     /// no expiration configured
//     None,
//     /// block height based expiry config
//     Block(u64),
//     /// timestamp based expiry config
//     Time(Timestamp),
// }

// impl ExpiryConfig {
//     pub fn get_response_attributes(&self) -> Vec<Attribute> {
//         match self {
//             ExpiryConfig::None => vec![Attribute::new("expiry_config", "none")],
//             ExpiryConfig::Block(h) => vec![Attribute::new(
//                 "expiry_config_expiry_block_height",
//                 h.to_string(),
//             )],
//             ExpiryConfig::Time(t) => vec![Attribute::new(
//                 "expiry_config_expiry_block_timestamp",
//                 t.to_string(),
//             )],
//         }
//     }

//     /// validates that the lockup config being stored is not already expired.
//     pub fn validate(&self, block_info: &BlockInfo) -> Result<(), StdError> {
//         match self {
//             ExpiryConfig::None => Ok(()),
//             ExpiryConfig::Block(h) => {
//                 if h > &block_info.height {
//                     Ok(())
//                 } else {
//                     Err(StdError::generic_err(
//                         "invalid expiry config: block height must be in the future".to_string(),
//                     ))
//                 }
//             }
//             ExpiryConfig::Time(t) => {
//                 if t.nanos() > block_info.time.nanos() {
//                     Ok(())
//                 } else {
//                     Err(StdError::generic_err(
//                         "invalid expiry config: block time must be in the future".to_string(),
//                     ))
//                 }
//             }
//         }
//     }

//     /// compares current block info with the stored expiry config.
//     /// returns false if no expiry configuration is stored.
//     /// otherwise, returns true if the current block is past the stored info.
//     pub fn is_expired(&self, block_info: BlockInfo) -> bool {
//         match self {
//             // no expiration date
//             ExpiryConfig::None => false,
//             // if stored expiration block height is less than or equal to the current block,
//             // expired
//             ExpiryConfig::Block(h) => h <= &block_info.height,
//             // if stored expiration timestamp is more than or equal to the current timestamp,
//             // expired
//             ExpiryConfig::Time(t) => t.nanos() <= block_info.time.nanos(),
//         }
//     }
// }

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
    pub native_denom: String,
    /// information about receiver address
    pub receiver_config: ReceiverConfig,
}

impl CovenantParty {
    pub fn get_refund_msg(self, amount: Uint128, block: &BlockInfo) -> CosmosMsg {
        match self.receiver_config {
            ReceiverConfig::Native(addr) => CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.to_string(),
                amount: vec![cosmwasm_std::Coin {
                    denom: self.native_denom,
                    amount,
                }],
            }),
            ReceiverConfig::Ibc(destination_config) => CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: destination_config.destination_chain_channel_id,
                to_address: self.addr.to_string(),
                amount: cosmwasm_std::Coin {
                    denom: self.native_denom,
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
            Attribute::new("party_a_ibc_denom", self.party_a.native_denom),
            Attribute::new("party_b_address", self.party_b.addr),
            Attribute::new("party_b_ibc_denom", self.party_b.native_denom),
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

pub fn default_ibc_ack_fee_amount() -> Uint128 {
    Uint128::new(100000)
}

pub fn default_ibc_timeout_fee_amount() -> Uint128 {
    Uint128::new(100000)
}

pub fn default_ibc_fee() -> IbcFee {
    IbcFee {
        // must be empty
        recv_fee: vec![],
        ack_fee: vec![cosmwasm_std::Coin {
            denom: "untrn".to_string(),
            amount: default_ibc_ack_fee_amount(),
        }],
        timeout_fee: vec![cosmwasm_std::Coin {
            denom: "untrn".to_string(),
            amount: default_ibc_timeout_fee_amount(),
        }],
    }
}

pub fn get_default_ibc_fee_requirement() -> Uint128 {
    default_ibc_ack_fee_amount() + default_ibc_timeout_fee_amount()
}

impl DestinationConfig {
    pub fn get_ibc_transfer_messages_for_coins(
        &self,
        coins: Vec<Coin>,
        current_timestamp: Timestamp,
        address: String,
    ) -> Vec<CosmosMsg<NeutronMsg>> {
        let mut messages: Vec<CosmosMsg<NeutronMsg>> = vec![];
        // we get the number of target denoms we have to reserve
        // neutron fees for. we pessimistically add 1 extra to
        // the count to enable one additional transfer if needed.
        let count = Uint128::from(1 + coins.len() as u128);

        for coin in coins {
            // if denom is not neutron, we just distribute it entirely
            let send_coin = if coin.denom != "untrn" {
                Some(coin)
            } else {
                // if its neutron we're distributing we need to keep a
                // reserve for ibc gas costs.
                // this is safe because we pass target denoms.
                let reserve_amount = count * get_default_ibc_fee_requirement();
                if coin.amount > reserve_amount {
                    Some(Coin {
                        denom: coin.denom,
                        amount: coin.amount - reserve_amount,
                    })
                } else {
                    None
                }
            };

            if let Some(c) = send_coin {
                messages.push(CosmosMsg::Custom(NeutronMsg::IbcTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: self.destination_chain_channel_id.to_string(),
                    token: c.clone(),
                    sender: address.to_string(),
                    receiver: self.destination_receiver_addr.to_string(),
                    timeout_height: RequestPacketTimeoutHeight {
                        revision_number: None,
                        revision_height: None,
                    },
                    timeout_timestamp: current_timestamp
                        .plus_seconds(self.ibc_transfer_timeout.u64())
                        .nanos(),
                    memo: format!("ibc_distribution: {:?}:{:?}", c.denom, c.amount,).to_string(),
                    fee: default_ibc_fee(),
                }))
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

/// queries the liquidity token balance of given address
pub fn query_liquidity_token_balance(
    querier: QuerierWrapper,
    liquidity_token: &str,
    contract_addr: String,
) -> Result<Uint128, StdError> {
    let liquidity_token_balance: BalanceResponse = querier.query_wasm_smart(
        liquidity_token,
        &cw20::Cw20QueryMsg::Balance {
            address: contract_addr,
        },
    )?;
    Ok(liquidity_token_balance.balance)
}

/// queries the cw20 liquidity token address corresponding to a given pool
pub fn query_liquidity_token_address(
    querier: QuerierWrapper,
    pool: String,
) -> Result<String, StdError> {
    let pair_info: PairInfo =
        querier.query_wasm_smart(pool, &astroport::pair::QueryMsg::Pair {})?;
    Ok(pair_info.liquidity_token.to_string())
}

pub fn query_astro_pool_token(
    querier: QuerierWrapper,
    pool: String,
    addr: String,
) -> Result<AstroportPoolTokenResponse, StdError> {
    let pair_info: PairInfo =
        querier.query_wasm_smart(pool, &astroport::pair::QueryMsg::Pair {})?;

    let liquidity_token_balance: BalanceResponse = querier.query_wasm_smart(
        pair_info.liquidity_token.as_ref(),
        &cw20::Cw20QueryMsg::Balance { address: addr },
    )?;

    Ok(AstroportPoolTokenResponse {
        pair_info,
        balance_response: liquidity_token_balance,
    })
}

#[cw_serde]
pub struct AstroportPoolTokenResponse {
    pub pair_info: PairInfo,
    pub balance_response: BalanceResponse,
}
