use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, QuerierWrapper, StdResult};
use neutron_sdk::{bindings::query::NeutronQuery, NeutronError};

use crate::error::ContractError;
use std::collections::HashSet;

/// Query next contract for the memo field
/// If query failed, we set memo to empty string, meaning no memo is expected
/// If query returns an empty string, we error out because we expect the memo not to be empty
///
/// We do that because not all next contract will need a memo, and if the next contract
/// doesn't have the NextMemo query, we don't want to error out, rather return an empty memo.
/// Thats why if the NextMemo query doesn't fail, we expect it to return a non-empty string.
///
/// This requires the next contract to implement the NextMemo query if it needs it, and
/// be careful not to return an error.
pub(crate) fn get_next_memo(
    querier: QuerierWrapper<NeutronQuery>,
    addr: &str,
) -> StdResult<String> {
    #[cw_serde]
    enum Query {
        NextMemo {},
    }

    // We check that the query was successful, if not, we return empty string
    let Ok(memo) = querier.query_wasm_smart::<String>(addr.to_string(), &Query::NextMemo {}) else {
        return Ok("".to_string());
    };

    // If the query was successful, we expect the memo to be non-empty
    // If memo is empty, something went wrong in the query, so we should error and retry later
    if memo.is_empty() {
        Err(cosmwasm_std::StdError::generic_err(
            "NextMemo query returned empty string",
        ))
    } else {
        Ok(memo)
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct IbcCounterpartyHeight {
    #[prost(uint64, optional, tag = "1")]
    revision_number: Option<u64>,
    #[prost(uint64, optional, tag = "2")]
    revision_height: Option<u64>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgTransfer {
    /// the port on which the packet will be sent
    #[prost(string, tag = "1")]
    pub source_port: String,
    /// the channel by which the packet will be sent
    #[prost(string, tag = "2")]
    pub source_channel: String,
    /// the tokens to be transferred
    #[prost(message, optional, tag = "3")]
    pub token: Option<cosmos_sdk_proto::cosmos::base::v1beta1::Coin>,
    /// the sender address
    #[prost(string, tag = "4")]
    pub sender: String,
    /// the recipient address on the destination chain
    #[prost(string, tag = "5")]
    pub receiver: String,
    /// Timeout height relative to the current block height.
    /// The timeout is disabled when set to 0.
    #[prost(message, optional, tag = "6")]
    pub timeout_height: Option<IbcCounterpartyHeight>,
    /// Timeout timestamp in absolute nanoseconds since unix epoch.
    /// The timeout is disabled when set to 0.
    #[prost(uint64, tag = "7")]
    pub timeout_timestamp: u64,
    #[prost(string, tag = "8")]
    pub memo: String,
}

pub fn validate_privileged_accounts(
    api: &dyn Api,
    privileged_accounts: Option<Vec<String>>,
) -> Result<Option<HashSet<Addr>>, ContractError> {
    privileged_accounts
        .map(|addresses| {
            ensure!(
                !addresses.is_empty(),
                ContractError::InvalidPrivilegedAccounts
            );

            addresses
                .iter()
                .map(|addr| {
                    api.addr_validate(addr)
                        .map_err(|_| ContractError::InvalidPrivilegedAccounts)
                })
                .collect::<Result<HashSet<_>, ContractError>>()
        })
        .transpose()
}

pub fn verify_caller(
    caller: &Addr,
    privileged_accounts: &Option<HashSet<Addr>>,
) -> Result<(), NeutronError> {
    if let Some(privileged_accounts) = privileged_accounts {
        if !privileged_accounts.contains(caller) {
            return Err(ContractError::Unauthorized {}.into());
        }
    }
    Ok(())
}
