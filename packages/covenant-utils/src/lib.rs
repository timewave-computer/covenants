use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_string, Addr, Api, Attribute, Coin, CosmosMsg, Decimal, StdError, StdResult, Timestamp,
    Uint128, Uint64,
};
use neutron::flatten_ibc_fee_total_amount;
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    sudo::msg::RequestPacketTimeoutHeight,
};

pub mod astroport;
pub mod deadline;
pub mod ica;
pub mod instantiate2_helper;
pub mod liquid_pooler_withdraw;
pub mod neutron;
pub mod op_mode;
pub mod polytone;
pub mod privileged_accounts;
pub mod split;
pub mod withdraw_lp_helper;

#[cw_serde]
pub struct InterchainCovenantParty {
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// connection id to the party chain
    pub party_chain_connection_id: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
    /// channel id from party to host chain
    pub party_to_host_chain_channel_id: String,
    /// channel id from host chain to the party chain
    pub host_to_party_chain_channel_id: String,
    /// denom provided by the party on its native chain
    pub remote_chain_denom: String,
    /// authorized address of the party on neutron
    pub addr: String,
    /// denom provided by the party on neutron
    pub native_denom: String,
    /// coin provided by the party on its native chain
    pub contribution: Coin,
    /// configuration for unwinding the denoms via pfm
    pub denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
    /// fallback refund address on the remote chain
    pub fallback_address: Option<String>,
}

#[cw_serde]
pub struct NativeCovenantParty {
    /// address of the receiver on destination chain
    pub party_receiver_addr: String,
    /// denom provided by the party on neutron
    pub native_denom: String,
    /// authorized address of the party on neutron
    pub addr: String,
    /// coin provided by the party on its native chain
    pub contribution: Coin,
}

#[cw_serde]
pub enum ReceiverConfig {
    /// party expects to receive funds on the same chain
    Native(String),
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
    pub fn validate_receiver_address(&self, api: &dyn Api) -> StdResult<Addr> {
        match &self.receiver_config {
            ReceiverConfig::Native(addr) => api.addr_validate(addr),
            ReceiverConfig::Ibc(destination_config) => {
                match soft_validate_remote_chain_addr(
                    api,
                    &destination_config.destination_receiver_addr,
                ) {
                    Ok(_) => Ok(Addr::unchecked(
                        &destination_config.destination_receiver_addr,
                    )),
                    Err(e) => Err(e),
                }
            }
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

    pub fn validate_party_addresses(&self, api: &dyn Api) -> StdResult<()> {
        self.party_a.validate_receiver_address(api)?;
        self.party_b.validate_receiver_address(api)?;
        Ok(())
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
    pub local_to_destination_chain_channel_id: String,
    /// address of the receiver on destination chain
    pub destination_receiver_addr: String,
    /// timeout in seconds
    pub ibc_transfer_timeout: Uint64,
    /// pfm configurations for denoms
    pub denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
}

#[cw_serde]
pub struct PacketForwardMiddlewareConfig {
    pub local_to_hop_chain_channel_id: String,
    pub hop_to_destination_chain_channel_id: String,
    pub hop_chain_receiver_address: String,
}

pub fn get_default_ica_fee() -> Coin {
    Coin {
        denom: "untrn".to_string(),
        amount: Uint128::new(1000000),
    }
}

// https://github.com/strangelove-ventures/packet-forward-middleware/blob/main/router/types/forward.go
#[cw_serde]
pub struct PacketMetadata {
    pub forward: Option<ForwardMetadata>,
}

#[cw_serde]
pub struct ForwardMetadata {
    pub receiver: String,
    pub port: String,
    pub channel: String,
}

impl DestinationConfig {
    pub fn get_ibc_transfer_messages_for_coins(
        &self,
        coins: Vec<Coin>,
        current_timestamp: Timestamp,
        sender_address: String,
        ibc_fee: IbcFee,
    ) -> StdResult<Vec<CosmosMsg<NeutronMsg>>> {
        let mut messages: Vec<CosmosMsg<NeutronMsg>> = vec![];
        // we get the number of target denoms we have to reserve
        // neutron fees for
        let count = Uint128::from(1 + coins.len() as u128);

        let total_fee = flatten_ibc_fee_total_amount(&ibc_fee);

        for coin in coins {
            let send_coin = if coin.denom != "untrn" {
                Some(coin)
            } else {
                // if its neutron we're distributing we need to keep a
                // reserve for ibc gas costs.
                // this is safe because we pass target denoms.
                let reserve_amount = count * total_fee;
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
                match self.denom_to_pfm_map.get(&c.denom) {
                    Some(pfm_config) => {
                        messages.push(CosmosMsg::Custom(NeutronMsg::IbcTransfer {
                            source_port: "transfer".to_string(),
                            // local chain to hop chain channel
                            source_channel: pfm_config.local_to_hop_chain_channel_id.to_string(),
                            token: c.clone(),
                            sender: sender_address.to_string(),
                            receiver: pfm_config.hop_chain_receiver_address.to_string(),
                            timeout_height: RequestPacketTimeoutHeight {
                                revision_number: None,
                                revision_height: None,
                            },
                            timeout_timestamp: current_timestamp
                                .plus_seconds(self.ibc_transfer_timeout.u64())
                                .nanos(),
                            memo: to_json_string(&PacketMetadata {
                                forward: Some(ForwardMetadata {
                                    receiver: self.destination_receiver_addr.to_string(),
                                    port: "transfer".to_string(),
                                    // hop chain to final receiver chain channel
                                    channel: pfm_config
                                        .hop_to_destination_chain_channel_id
                                        .to_string(),
                                }),
                            })?,
                            fee: ibc_fee.clone(),
                        }))
                    }
                    None => {
                        messages.push(CosmosMsg::Custom(NeutronMsg::IbcTransfer {
                            source_port: "transfer".to_string(),
                            source_channel: self.local_to_destination_chain_channel_id.to_string(),
                            token: c.clone(),
                            sender: sender_address.to_string(),
                            receiver: self.destination_receiver_addr.to_string(),
                            timeout_height: RequestPacketTimeoutHeight {
                                revision_number: None,
                                revision_height: None,
                            },
                            timeout_timestamp: current_timestamp
                                .plus_seconds(self.ibc_transfer_timeout.u64())
                                .nanos(),
                            memo: format!("ibc_distribution: {:?}:{:?}", c.denom, c.amount,)
                                .to_string(),
                            fee: ibc_fee.clone(),
                        }));
                    }
                }
            }
        }

        Ok(messages)
    }

    pub fn get_response_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new(
                "local_to_destination_chain_channel_id",
                self.local_to_destination_chain_channel_id.to_string(),
            ),
            Attribute::new(
                "destination_receiver_addr",
                self.destination_receiver_addr.to_string(),
            ),
            Attribute::new("ibc_transfer_timeout", self.ibc_transfer_timeout),
        ]
    }
}

#[cw_serde]
pub struct PfmUnwindingConfig {
    // keys: relevant denoms IBC'd to neutron
    // values: channel ids to facilitate ibc unwinding to party chain
    pub party_1_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
    pub party_2_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
}

/// single side lp limits define the highest amount (in `Uint128`) that
/// we consider acceptable to provide single-sided.
/// if asset balance exceeds these limits, double-sided liquidity should be provided.
#[cw_serde]
pub struct SingleSideLpLimits {
    pub asset_a_limit: Uint128,
    pub asset_b_limit: Uint128,
}

/// config for the pool price expectations upon covenant instantiation
#[cw_serde]
pub struct PoolPriceConfig {
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
}

/// soft validation for addresses on remote chains.
/// skips the bech32 prefix and variant checks.
pub fn soft_validate_remote_chain_addr(api: &dyn Api, addr: &str) -> StdResult<()> {
    let (_prefix, decoded, _variant) = bech32::decode(addr).map_err(|e| {
        StdError::generic_err(format!(
            "soft_addr_validation for address {:?} failed to bech32 decode: {:?}",
            addr,
            e.to_string()
        ))
    })?;
    let decoded_bytes = <Vec<u8> as bech32::FromBase32>::from_base32(&decoded).map_err(|e| {
        StdError::generic_err(format!(
            "soft_addr_validation for address {:?} failed to get bytes from base32: {:?}",
            addr,
            e.to_string()
        ))
    })?;

    match api.addr_humanize(&decoded_bytes.into()) {
        Ok(_) => Ok(()),
        Err(e) => Err(StdError::generic_err(format!(
            "soft_addr_validation for address {:?} failed to addr_humanize: {:?}",
            addr,
            e.to_string()
        ))),
    }
}
