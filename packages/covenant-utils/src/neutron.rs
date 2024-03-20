use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Attribute, Binary, MessageInfo, QuerierWrapper, StdError, StdResult, Uint128, Uint64,
};
use cw_utils::must_pay;
use neutron_sdk::{
    bindings::{msg::IbcFee, query::NeutronQuery, types::ProtobufAny},
    query::min_ibc_fee::MinIbcFeeResponse,
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
}

impl RemoteChainInfo {
    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new("connection_id", &self.connection_id),
            Attribute::new("channel_id", &self.channel_id),
            Attribute::new("denom", &self.denom),
            Attribute::new(
                "ibc_transfer_timeout",
                self.ibc_transfer_timeout.to_string(),
            ),
            Attribute::new("ica_timeout", self.ica_timeout.to_string()),
        ]
    }
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum CovenantQueryMsg {
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

#[cw_serde]
pub struct MinIbcFeeConfig {
    pub ibc_fee: IbcFee,
    pub total_ntrn_fee: Uint128,
}

pub fn query_ibc_fee(querier: QuerierWrapper<'_, NeutronQuery>) -> StdResult<MinIbcFeeConfig> {
    let min_fee_query_response: MinIbcFeeResponse =
        querier.query(&NeutronQuery::MinIbcFee {}.into())?;
    let total_fee_amount = flatten_ibc_fee_total_amount(&min_fee_query_response.min_fee);

    Ok(MinIbcFeeConfig {
        ibc_fee: min_fee_query_response.min_fee,
        total_ntrn_fee: total_fee_amount,
    })
}

pub fn flatten_ibc_fee_total_amount(ibc_fee: &IbcFee) -> Uint128 {
    let mut total_amount = Uint128::zero();

    for coin in &ibc_fee.recv_fee {
        total_amount += coin.amount;
    }

    for coin in &ibc_fee.ack_fee {
        total_amount += coin.amount;
    }

    for coin in &ibc_fee.timeout_fee {
        total_amount += coin.amount;
    }

    total_amount
}

/// assertion helper that checks if the caller has covered ibc fees
/// for `count` number of transactions
pub fn assert_ibc_fee_coverage(
    info: MessageInfo,
    total_fee: Uint128,
    count: Uint128,
) -> StdResult<()> {
    // the caller must cover the ibc fees
    match must_pay(&info, "untrn") {
        Ok(amt) => {
            if amt < total_fee.checked_mul(count)? {
                Err(StdError::generic_err("insufficient fees"))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(StdError::generic_err(
            "must cover ibc fees to distribute fallback denoms",
        )),
    }
}
