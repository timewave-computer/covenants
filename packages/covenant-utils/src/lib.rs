
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
}
