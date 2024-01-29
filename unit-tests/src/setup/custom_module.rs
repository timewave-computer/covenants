use std::str::FromStr;

use cosmwasm_std::{
    coin, coins, from_json, to_json_binary, to_json_string, Addr, BankMsg, StdError, StdResult,
    Storage, Uint128,
};
use covenant_ibc_forwarder::helpers::MsgTransfer;
use covenant_stride_liquid_staker::helpers::Autopilot;
use cw_multi_test::{
    addons::MockApiBech32,
    error::{bail, AnyError},
    prefixed_storage::{prefixed, prefixed_read},
    AppResponse, BankSudo, Module, WasmSudo,
};
use cw_storage_plus::Map;
use neutron_sdk::{
    bindings::{
        msg::{MsgSubmitTxResponse, NeutronMsg},
        query::NeutronQuery,
    },
    interchain_txs::helpers::get_port_id,
    sudo::msg::RequestPacket,
};
use prost::Message;

pub const CHAIN_PREFIX: &str = "cosmos";

/// Namespace for neutron storage
pub const NAMESPACE_NEUTRON: &[u8] = b"neutron_storage";

/// Map for (sender, conn_id) => account_id
const ACCOUNTS: Map<(&Addr, String, String), Addr> = Map::new("accounts");

const LOCAL_CHANNELS: Map<String, String> = Map::new("local_channels");
const LOCAL_CHANNELS_VALUES: Map<String, String> = Map::new("local_channels_values");

const REMOTE_CHANNELS: Map<String, String> = Map::new("remote_channels");
const REMOTE_CHANNELS_VALUES: Map<String, String> = Map::new("remote_channels_values");

pub trait Neutron:
    Module<ExecT = NeutronMsg, QueryT = NeutronQuery, SudoT = neutron_sdk::sudo::msg::SudoMsg>
{
}

pub struct NeutronKeeper {
    api: MockApiBech32,
    account_timeout: bool,
}

impl Neutron for NeutronKeeper {}

impl NeutronKeeper {
    pub fn new(prefix: &'static str) -> Self {
        Self {
            api: MockApiBech32::new(prefix),
            account_timeout: false,
        }
    }

    /// Sets our timeout flag, so the next message will return a timeout response instead of a successful response
    pub fn set_timeout(&mut self, timeout: bool) {
        self.account_timeout = timeout;
    }

    pub fn add_local_channel(
        &mut self,
        storage: &mut dyn Storage,
        source_channel: &str,
        other_channel: &str,
    ) -> Result<(), StdError> {
        LOCAL_CHANNELS.save(
            storage,
            source_channel.to_string(),
            &other_channel.to_string(),
        )?;
        LOCAL_CHANNELS_VALUES.save(
            storage,
            other_channel.to_string(),
            &source_channel.to_string(),
        )?;
        Ok(())
    }

    pub fn add_remote_channel(
        &mut self,
        storage: &mut dyn Storage,
        some_channel: &str,
        other_channel: &str,
    ) -> Result<(), StdError> {
        REMOTE_CHANNELS.save(
            storage,
            some_channel.to_string(),
            &other_channel.to_string(),
        )?;
        REMOTE_CHANNELS_VALUES.save(
            storage,
            other_channel.to_string(),
            &some_channel.to_string(),
        )?;
        Ok(())
    }
}

impl NeutronKeeper {
    fn register_account(
        &self,
        storage: &mut dyn Storage,
        sender: Addr,
        conn_id: String,
        account_id: String,
    ) -> Result<(), AnyError> {
        let mut ntrn_storage = prefixed(storage, NAMESPACE_NEUTRON);

        if ACCOUNTS.has(
            &mut ntrn_storage,
            (&sender, conn_id.clone(), account_id.clone()),
        ) {
            bail!("Account already registered");
        }

        let addr = self
            .api
            .addr_make(format!("{sender}_{conn_id}_{account_id}").as_str());

        ACCOUNTS
            .save(
                &mut ntrn_storage,
                (&sender, conn_id.clone(), account_id.clone()),
                &addr,
            )
            .unwrap();
        Ok(())
    }

    fn remove_account(
        &self,
        storage: &mut dyn Storage,
        sender: &Addr,
        conn_id: String,
        account_id: String,
    ) {
        let mut ntrn_storage = prefixed(storage, NAMESPACE_NEUTRON);

        ACCOUNTS.remove(&mut ntrn_storage, (sender, conn_id, account_id))
    }

    fn get_account(
        &self,
        storage: &dyn Storage,
        sender: &Addr,
        conn_id: &str,
        account_id: &str,
    ) -> StdResult<Addr> {
        let ntrn_storage = prefixed_read(storage, NAMESPACE_NEUTRON);

        ACCOUNTS.load(
            &ntrn_storage,
            (sender, conn_id.to_string(), account_id.to_string()),
        )
    }
}

impl Module for NeutronKeeper {
    type ExecT = NeutronMsg;
    type QueryT = NeutronQuery;
    type SudoT = neutron_sdk::sudo::msg::SudoMsg;

    /// Currently we only implement register ICA and ibcTransfer and SubmitTx,
    /// maybe we should implement other stuff as well?
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn cosmwasm_std::Storage,
        router: &dyn cw_multi_test::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        sender: cosmwasm_std::Addr,
        msg: Self::ExecT,
    ) -> cw_multi_test::error::AnyResult<cw_multi_test::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + cosmwasm_schema::schemars::JsonSchema
            + cosmwasm_schema::serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + cosmwasm_schema::serde::de::DeserializeOwned + 'static,
    {
        match msg {
            NeutronMsg::RegisterInterchainAccount {
                connection_id,
                interchain_account_id,
                register_fee,
            } => {
                // Send fees to fee burner
                // we do it mainly to make sure fees are deducted in our tests
                let fee = match register_fee {
                    Some(fee) => fee,
                    None => bail!("No register fee specified"),
                };

                let fee_msg = cosmwasm_std::BankMsg::Burn { amount: fee };

                router.execute(api, storage, block, sender.clone(), fee_msg.into())?;

                // Save the account in our storage for later use
                self.register_account(
                    storage,
                    sender.clone(),
                    connection_id.clone(),
                    interchain_account_id.clone(),
                )?;

                // Complete the registration by calling the sudo entry on the contract
                router.sudo(
                    api,
                    storage,
                    block,
                    cw_multi_test::SudoMsg::Wasm(WasmSudo {
                        contract_addr: sender.clone(),
                        msg: to_json_binary(&neutron_sdk::sudo::msg::SudoMsg::OpenAck {
                            port_id: get_port_id(sender.to_string(), interchain_account_id.clone()),
                            channel_id: "channel-1".to_string(),
                            counterparty_channel_id: "channel-1".to_string(),
                            counterparty_version: to_json_string(
                                &covenant_utils::neutron_ica::OpenAckVersion {
                                    version: "ica".to_string(),
                                    controller_connection_id: connection_id.clone(),
                                    host_connection_id: connection_id.clone(),
                                    address: self
                                        .api
                                        .addr_make(
                                            format!(
                                                "{sender}_{connection_id}_{interchain_account_id}"
                                            )
                                            .as_str(),
                                        )
                                        .to_string(),
                                    encoding: "encoding".to_string(),
                                    tx_type: "tx_type".to_string(),
                                },
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                    }),
                )?;

                Ok(AppResponse::default())
            }
            // TODO: Handle multiple PFM hops
            NeutronMsg::IbcTransfer {
                source_port: _,
                source_channel,
                token,
                sender: local_sender,
                receiver,
                timeout_height: _,
                timeout_timestamp: _,
                memo,
                fee,
            } => {
                let local_sender = api.addr_validate(&local_sender)?;

                // Burn fees first
                router.execute(
                    api,
                    storage,
                    block,
                    local_sender.clone(),
                    BankMsg::Burn {
                        amount: fee.ack_fee,
                    }
                    .into(),
                )?;

                // Get the counterparty channel of the local channel if it exists
                let Ok(counterparty_channel) = LOCAL_CHANNELS.load(storage, source_channel.clone())
                else {
                    bail!("Local channel doesn't exist")
                };

                // Handle the denom to include or remove the correct prefix
                let mut handled_denom = match try_pop_denom_prefix(&source_channel, &token.denom) {
                    Some(poped_denom) => poped_denom.to_string(),
                    None => format!("{counterparty_channel}/{}", token.denom),
                };

                // Handle pfm stuff here, mainly handle denom and get the receiver
                let receiver = if let Ok(pfm) = from_json::<covenant_utils::PacketMetadata>(memo) {
                    match pfm.forward {
                        Some(forward) => {
                            let new_prefix =
                                if LOCAL_CHANNELS_VALUES.has(storage, forward.channel.clone()) {
                                    let local_string = LOCAL_CHANNELS_VALUES
                                        .load(storage, forward.channel.clone())?;

                                    // Make sure the current string doesn't equal to the string we send on,
                                    if local_string == source_channel {
                                        bail!("PFM target channel is equal to the sending channel")
                                    }

                                    local_string
                                } else if REMOTE_CHANNELS.has(storage, forward.channel.clone()) {
                                    REMOTE_CHANNELS.load(storage, forward.channel.clone())?
                                } else {
                                    REMOTE_CHANNELS_VALUES.load(storage, forward.channel.clone())?
                                };

                            // Add prefix if needed
                            handled_denom =
                                match try_pop_denom_prefix(&forward.channel, &handled_denom) {
                                    Some(poped_denom) => poped_denom.to_string(),
                                    None => format!("{new_prefix}/{}", handled_denom),
                                };

                            forward.receiver
                        }
                        None => receiver,
                    }
                } else {
                    receiver
                };

                // Burn the existing tokens
                router.execute(
                    api,
                    storage,
                    block,
                    local_sender.clone(),
                    BankMsg::Burn {
                        amount: vec![token.clone()],
                    }
                    .into(),
                )?;

                // We mint the IBC token to the sender, so we can do a transfer alter
                router.sudo(
                    api,
                    storage,
                    block,
                    BankSudo::Mint {
                        to_address: local_sender.to_string(),
                        amount: coins(token.amount.u128(), handled_denom.clone()),
                    }
                    .into(),
                )?;

                // Do the bank transfer
                router.execute(
                    api,
                    storage,
                    block,
                    local_sender,
                    BankMsg::Send {
                        to_address: receiver,
                        amount: coins(token.amount.u128(), handled_denom),
                    }
                    .into(),
                )?;

                Ok(AppResponse::default())
            }
            NeutronMsg::SubmitTx {
                connection_id,
                interchain_account_id,
                msgs,
                memo: _,
                timeout: _,
                fee,
            } => {
                // Return timeout response if we have a timeout flag
                if self.account_timeout {
                    router.sudo(
                        api,
                        storage,
                        block,
                        cw_multi_test::SudoMsg::Wasm(WasmSudo {
                            contract_addr: sender.clone(),
                            msg: to_json_binary(&neutron_sdk::sudo::msg::SudoMsg::Timeout {
                                request: RequestPacket {
                                    sequence: Some(1),
                                    source_port: None,
                                    source_channel: Some("some_channel".to_string()),
                                    destination_port: None,
                                    destination_channel: None,
                                    data: None,
                                    timeout_height: None,
                                    timeout_timestamp: None,
                                },
                            })
                            .unwrap(),
                        }),
                    )?;

                    self.remove_account(storage, &sender, connection_id, interchain_account_id);

                    return Ok(AppResponse {
                        data: Some(
                            to_json_binary(&MsgSubmitTxResponse {
                                sequence_id: 1,
                                channel: "some_channel".to_string(),
                            })
                            .unwrap(),
                        ),
                        events: vec![],
                    });
                }

                let account = self.get_account(
                    storage,
                    &sender,
                    connection_id.as_str(),
                    interchain_account_id.as_str(),
                )?;

                // // Burn fees first
                router.execute(
                    api,
                    storage,
                    block,
                    sender.clone(),
                    BankMsg::Burn {
                        amount: fee.ack_fee,
                    }
                    .into(),
                )?;

                for msg in msgs {
                    match msg.type_url.as_str() {
                        "/ibc.applications.transfer.v1.MsgTransfer" => {
                            let msg: MsgTransfer =
                                Message::decode(msg.value.clone().as_slice()).unwrap();

                            let token = match msg.token {
                                Some(token) => Ok(coin(
                                    Uint128::from_str(token.amount.as_str()).unwrap().u128(),
                                    token.denom,
                                )),
                                None => Err(StdError::generic_err("No token specified")),
                            }
                            .unwrap();

                            let prefix =
                                if LOCAL_CHANNELS_VALUES.has(storage, msg.source_channel.clone()) {
                                    LOCAL_CHANNELS_VALUES
                                        .load(storage, msg.source_channel.clone())
                                        .unwrap()
                                } else if REMOTE_CHANNELS.has(storage, msg.source_channel.clone()) {
                                    REMOTE_CHANNELS
                                        .load(storage, msg.source_channel.clone())
                                        .unwrap()
                                } else {
                                    REMOTE_CHANNELS_VALUES
                                        .load(storage, msg.source_channel.clone())
                                        .unwrap()
                                };

                            let handled_denom = match try_pop_denom_prefix(
                                &msg.source_channel,
                                token.denom.as_str(),
                            ) {
                                Some(denom) => denom.to_string(),
                                None => format!("{prefix}/{}", token.denom),
                            };

                            // TODO: Handle PFM
                            let mut receiver = msg.receiver;

                            // We handle stride autopilot here, no denom change to ls token
                            // we just assume that if the token passed stride, it's a ls token
                            // This allows us to skip staking/unstaking the token, and just assume
                            // the ibc token is the ls token
                            if let Ok(autopilot) = from_json::<Autopilot>(msg.memo) {
                                receiver = autopilot.autopilot.stakeibc.stride_address;
                            }

                            // Burn the existing tokens
                            if let Err(err) = router.execute(
                                api,
                                storage,
                                block,
                                Addr::unchecked(account.clone()),
                                BankMsg::Burn {
                                    amount: vec![token.clone()],
                                }
                                .into(),
                            ) {
                                router
                                    .sudo(
                                        api,
                                        storage,
                                        block,
                                        cw_multi_test::SudoMsg::Wasm(WasmSudo {
                                            contract_addr: sender.clone(),
                                            msg: to_json_binary(
                                                &neutron_sdk::sudo::msg::SudoMsg::Error {
                                                    request: RequestPacket {
                                                        sequence: Some(1),
                                                        source_port: None,
                                                        source_channel: Some(
                                                            "some_channel".to_string(),
                                                        ),
                                                        destination_port: None,
                                                        destination_channel: None,
                                                        data: None,
                                                        timeout_height: None,
                                                        timeout_timestamp: None,
                                                    },
                                                    details: err.to_string(),
                                                },
                                            )
                                            .unwrap(),
                                        }),
                                    )
                                    .unwrap();

                                return Ok(AppResponse {
                                    data: Some(
                                        to_json_binary(&MsgSubmitTxResponse {
                                            sequence_id: 1,
                                            channel: "some_channel".to_string(),
                                        })
                                        .unwrap(),
                                    ),
                                    events: vec![],
                                });
                            };

                            // We mint the IBC token to the sender, so we can do a transfer later
                            router
                                .sudo(
                                    api,
                                    storage,
                                    block,
                                    BankSudo::Mint {
                                        to_address: sender.to_string(),
                                        amount: coins(token.amount.u128(), handled_denom.clone()),
                                    }
                                    .into(),
                                )
                                .unwrap();

                            // Do the bank transfer
                            router
                                .execute(
                                    api,
                                    storage,
                                    block,
                                    sender.clone(),
                                    BankMsg::Send {
                                        to_address: receiver,
                                        amount: coins(token.amount.u128(), handled_denom),
                                    }
                                    .into(),
                                )
                                .unwrap();

                            Ok(())
                        }

                        "/cosmos.bank.v1beta1.MsgSend" => todo!(),
                        "/cosmos.bank.v1beta1.MsgMultiSend" => todo!(),
                        _ => Err(StdError::generic_err("Unknown message type")),
                    }
                    .unwrap();
                }

                // Complete the registration by calling the sudo entry on the contract
                router.sudo(
                    api,
                    storage,
                    block,
                    cw_multi_test::SudoMsg::Wasm(WasmSudo {
                        contract_addr: sender.clone(),
                        msg: to_json_binary(&neutron_sdk::sudo::msg::SudoMsg::Response {
                            request: RequestPacket {
                                sequence: Some(1),
                                source_port: None,
                                source_channel: Some("some_channel".to_string()),
                                destination_port: None,
                                destination_channel: None,
                                data: Some(to_json_binary("").unwrap()),
                                timeout_height: None,
                                timeout_timestamp: None,
                            },
                            data: to_json_binary("").unwrap(),
                        })
                        .unwrap(),
                    }),
                )?;

                Ok(AppResponse {
                    data: Some(
                        to_json_binary(&MsgSubmitTxResponse {
                            sequence_id: 1,
                            channel: "some_channel".to_string(),
                        })
                        .unwrap(),
                    ),
                    events: vec![],
                })
            }

            NeutronMsg::RegisterInterchainQuery { .. } => unimplemented!(),
            NeutronMsg::UpdateInterchainQuery { .. } => unimplemented!(),
            NeutronMsg::RemoveInterchainQuery { .. } => unimplemented!(),
            NeutronMsg::SubmitAdminProposal { .. } => unimplemented!(),
            NeutronMsg::CreateDenom { .. } => unimplemented!(),
            NeutronMsg::ChangeAdmin { .. } => unimplemented!(),
            NeutronMsg::MintTokens { .. } => unimplemented!(),
            NeutronMsg::BurnTokens { .. } => unimplemented!(),
            NeutronMsg::SetBeforeSendHook { .. } => unimplemented!(),
            NeutronMsg::AddSchedule { .. } => unimplemented!(),
            NeutronMsg::RemoveSchedule { .. } => unimplemented!(),
            NeutronMsg::ResubmitFailure { .. } => unimplemented!(),
            NeutronMsg::Dex(_) => unimplemented!(),
        }
    }

    fn query(
        &self,
        _api: &dyn cosmwasm_std::Api,
        storage: &dyn cosmwasm_std::Storage,
        _querier: &dyn cosmwasm_std::Querier,
        _block: &cosmwasm_std::BlockInfo,
        request: Self::QueryT,
    ) -> cw_multi_test::error::AnyResult<cosmwasm_std::Binary> {
        match request {
            NeutronQuery::InterchainAccountAddress {
                owner_address,
                interchain_account_id,
                connection_id,
            } => Ok(to_json_binary(
                &self
                    .get_account(
                        storage,
                        &Addr::unchecked(owner_address),
                        connection_id.as_str(),
                        interchain_account_id.as_str(),
                    )
                    .unwrap(),
            )
            .unwrap()),
            NeutronQuery::MinIbcFee {} => todo!(),

            NeutronQuery::InterchainQueryResult { .. } => todo!(),
            NeutronQuery::RegisteredInterchainQueries { .. } => todo!(),
            NeutronQuery::RegisteredInterchainQuery { .. } => todo!(),
            NeutronQuery::TotalBurnedNeutronsAmount {} => todo!(),
            NeutronQuery::FullDenom { .. } => todo!(),
            NeutronQuery::DenomAdmin { .. } => todo!(),
            NeutronQuery::BeforeSendHook { .. } => todo!(),
            NeutronQuery::Failures { .. } => todo!(),
            NeutronQuery::Dex(_) => todo!(),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn cw_multi_test::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _msg: Self::SudoT,
    ) -> cw_multi_test::error::AnyResult<cw_multi_test::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + cosmwasm_schema::schemars::JsonSchema
            + cosmwasm_schema::serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + cosmwasm_schema::serde::de::DeserializeOwned + 'static,
    {
        bail!("No sudo messages")
    }
}

/// Try to remove the channel prefix from a denom
/// If removed, it means denom came from the channel (ibc denom)
/// If not removed, it means the denom didn't came from the channel
fn try_pop_denom_prefix<'a>(prefix: &'a str, denom: &'a str) -> Option<&'a str> {
    denom.strip_prefix(format!("{prefix}/").as_str())
}
