
pub mod neutron_ica {
    use cosmwasm_schema::{cw_serde, QueryResponses};
    use cosmwasm_std::{Uint64, Addr, Binary, StdError};
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

    #[cw_serde]
    #[derive(QueryResponses)]
    pub enum QueryMsg {
        /// Returns the associated remote chain information
        #[returns(Option<Addr>)]
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
