use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, Coin, CosmosMsg, Decimal, StdError, StdResult,
    Uint128, Uint64, WasmMsg,
};
use covenant_macros::{
    clocked, covenant_clock_address, covenant_deposit_address, covenant_lper_withdraw,
};
use covenant_utils::{OutpostExecuteMsg, ForwardMetadata};
use cw_utils::Expiration;
use polytone::callbacks::CallbackMessage;

#[cw_serde]
pub struct InstantiateMsg {
    pub clock_address: String,
    pub holder_address: String,
    pub note_address: String,
    pub pool_id: Uint64,
    pub osmo_ibc_timeout: Uint64,
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub osmo_outpost: String,
    pub lp_token_denom: String,
    pub slippage_tolerance: Option<Decimal>,
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
    pub funding_duration_seconds: Uint64,
}

#[cw_serde]
pub struct PresetOsmoLiquidPoolerFields {
    pub label: String,
    pub code_id: u64,
    pub note_address: String,
    pub pool_id: Uint64,
    pub osmo_ibc_timeout: Uint64,
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub osmo_outpost: String,
    pub lp_token_denom: String,
    pub slippage_tolerance: Option<Decimal>,
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
    pub funding_duration_seconds: Uint64,
}

impl PresetOsmoLiquidPoolerFields {
    pub fn to_instantiate_msg(
        &self,
        clock_address: String,
        holder_address: String,
    ) -> InstantiateMsg {
        InstantiateMsg {
            clock_address,
            holder_address,
            note_address: self.note_address.to_string(),
            pool_id: self.pool_id,
            osmo_ibc_timeout: self.osmo_ibc_timeout,
            party_1_chain_info: self.party_1_chain_info.clone(),
            party_2_chain_info: self.party_2_chain_info.clone(),
            osmo_to_neutron_channel_id: self.osmo_to_neutron_channel_id.to_string(),
            party_1_denom_info: self.party_1_denom_info.clone(),
            party_2_denom_info: self.party_2_denom_info.clone(),
            osmo_outpost: self.osmo_outpost.to_string(),
            lp_token_denom: self.lp_token_denom.to_string(),
            slippage_tolerance: self.slippage_tolerance,
            expected_spot_price: self.expected_spot_price,
            acceptable_price_spread: self.acceptable_price_spread,
            funding_duration_seconds: self.funding_duration_seconds,
        }
    }

    pub fn to_instantiate2_msg(
        &self,
        admin_addr: String,
        salt: Binary,
        clock_address: String,
        holder_address: String,
    ) -> Result<WasmMsg, StdError> {
        Ok(WasmMsg::Instantiate2 {
            admin: Some(admin_addr),
            code_id: self.code_id,
            label: self.label.to_string(),
            msg: to_json_binary(&self.to_instantiate_msg(clock_address, holder_address))?,
            funds: vec![],
            salt,
        })
    }
}

#[cw_serde]
pub struct LiquidityProvisionConfig {
    pub latest_balances: HashMap<String, Coin>,
    pub party_1_denom_info: PartyDenomInfo,
    pub party_2_denom_info: PartyDenomInfo,
    pub pool_id: Uint64,
    pub outpost: String,
    pub lp_token_denom: String,
    pub slippage_tolerance: Option<Decimal>,
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
    pub funding_duration_seconds: Uint64,
}

#[cw_serde]
pub struct IbcConfig {
    pub party_1_chain_info: PartyChainInfo,
    pub party_2_chain_info: PartyChainInfo,
    pub osmo_to_neutron_channel_id: String,
    pub osmo_ibc_timeout: Uint64,
}

impl IbcConfig {
    pub fn to_response_attributes(self) -> Vec<Attribute> {
        let mut attributes = vec![
            Attribute::new(
                "osmo_to_neutron_channel_id",
                self.osmo_to_neutron_channel_id,
            ),
            Attribute::new("osmo_ibc_timeout", self.osmo_ibc_timeout.to_string()),
        ];
        attributes.extend(
            self.party_1_chain_info
                .to_response_attributes("party_1".to_string()),
        );
        attributes.extend(
            self.party_2_chain_info
                .to_response_attributes("party_2".to_string()),
        );

        attributes
    }
}

impl LiquidityProvisionConfig {
    pub fn get_party_1_proxy_balance(&self) -> Option<&Coin> {
        self.latest_balances
            .get(&self.party_1_denom_info.osmosis_coin.denom)
    }

    pub fn get_party_2_proxy_balance(&self) -> Option<&Coin> {
        self.latest_balances
            .get(&self.party_2_denom_info.osmosis_coin.denom)
    }

    pub fn get_osmo_outpost_provide_liquidity_message(&self) -> StdResult<CosmosMsg> {
        let mut funds = vec![];
        if let Some(c) = self.get_party_1_proxy_balance() {
            funds.push(c.clone());
        }
        if let Some(c) = self.get_party_2_proxy_balance() {
            funds.push(c.clone());
        }

        let outpost_config = covenant_utils::OutpostProvideLiquidityConfig {
            pool_id: Uint64::new(self.pool_id.u64()),
            expected_spot_price: self.expected_spot_price,
            acceptable_price_spread: self.acceptable_price_spread,
            // if no slippage tolerance is passed, we use 0
            slippage_tolerance: self.slippage_tolerance.unwrap_or_default(),
            asset_1_single_side_lp_limit: self.party_1_denom_info.single_side_lp_limit,
            asset_2_single_side_lp_limit: self.party_2_denom_info.single_side_lp_limit,
        };

        Ok(WasmMsg::Execute {
            contract_addr: self.outpost.to_string(),
            msg: to_json_binary(&OutpostExecuteMsg::ProvideLiquidity {
                config: outpost_config,
            })?,
            funds,
        }
        .into())
    }

    pub fn reset_latest_proxy_balances(&mut self) {
        self.latest_balances
            .remove(&self.party_1_denom_info.osmosis_coin.denom);
        self.latest_balances
            .remove(&self.party_2_denom_info.osmosis_coin.denom);
        self.latest_balances
            .remove(&self.lp_token_denom);
    }

    pub fn proxy_received_party_contributions(&self, p1_coin: &Coin, p2_coin: &Coin) -> bool {
        let p1_funded = p1_coin.amount >= self.party_1_denom_info.get_osmo_bal();
        let p2_funded = p2_coin.amount >= self.party_2_denom_info.get_osmo_bal();
        p1_funded && p2_funded
    }

    pub fn to_response_attributes(self) -> Vec<Attribute> {
        let slippage_tolerance = match self.slippage_tolerance {
            Some(val) => val.to_string(),
            None => "None".to_string(),
        };
        let proxy_bals: Vec<Attribute> = self
            .latest_balances
            .iter()
            .map(|(denom, coin)| Attribute::new(denom, coin.to_string()))
            .collect();
        let mut attributes = vec![
            Attribute::new("pool_id", self.pool_id.to_string()),
            Attribute::new("outpost", self.outpost),
            Attribute::new("lp_token_denom", self.lp_token_denom),
            Attribute::new("slippage_tolerance", slippage_tolerance),
            Attribute::new("expected_spot_price", self.expected_spot_price.to_string()),
            Attribute::new(
                "acceptable_price_spread",
                self.acceptable_price_spread.to_string(),
            ),
        ];
        attributes.extend(
            self.party_1_denom_info
                .to_response_attributes("party_1".to_string()),
        );
        attributes.extend(
            self.party_1_denom_info
                .to_response_attributes("party_2".to_string()),
        );
        attributes.extend(proxy_bals);

        attributes
    }
}

#[cw_serde]
pub struct PartyDenomInfo {
    /// coin as denominated on osmosis
    pub osmosis_coin: Coin,
    /// ibc denom on liquid pooler chain
    pub local_denom: String,
    /// the max amount of tokens allow to be single-side lp'd
    pub single_side_lp_limit: Uint128,
}

impl PartyDenomInfo {
    pub fn get_osmo_bal(&self) -> Uint128 {
        self.osmosis_coin.amount
    }

    pub fn to_response_attributes(&self, party: String) -> Vec<Attribute> {
        vec![
            Attribute {
                key: format!("{:?}_neutron_denom", party),
                value: self.local_denom.to_string(),
            },
            Attribute {
                key: format!("{:?}_osmosis_coin", party),
                value: self.osmosis_coin.to_string(),
            },
            Attribute {
                key: format!("{:?}_single_side_lp_limit", party),
                value: self.single_side_lp_limit.to_string(),
            },
        ]
    }
}

#[clocked]
#[covenant_lper_withdraw]
#[cw_serde]
pub enum ExecuteMsg {
    // polytone callback listener
    Callback(CallbackMessage),
}

#[covenant_clock_address]
#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractState)]
    ContractState {},
    #[returns(Addr)]
    HolderAddress {},
    #[returns(LiquidityProvisionConfig)]
    LiquidityProvisionConfig {},
    #[returns(IbcConfig)]
    IbcConfig {},
    #[returns(Option<String>)]
    ProxyAddress {},
    #[returns(Vec<String>)]
    Callbacks {},
}

/// state of the LP state machine
#[cw_serde]
pub enum ContractState {
    Instantiated,
    ProxyCreated,
    ProxyFunded { funding_expiration: Expiration },
    Active,
    Distributing { coins: Vec<Coin> },
}

#[cw_serde]
pub struct PartyChainInfo {
    pub neutron_to_party_chain_channel: String,
    pub party_chain_to_neutron_channel: String,
    pub outwards_pfm: Option<ForwardMetadata>,
    pub inwards_pfm: Option<ForwardMetadata>,
    pub ibc_timeout: Uint64,
}

impl PartyChainInfo {
    pub fn to_response_attributes(&self, party: String) -> Vec<Attribute> {
        let pfm_attributes: Vec<Attribute> = match &self.outwards_pfm {
            Some(val) => {
                vec![
                    Attribute::new(
                        format!("{:?}_pfm_receiver", party),
                        val.receiver.to_string(),
                    ),
                    Attribute::new(format!("{:?}_pfm_port", party), val.port.to_string()),
                    Attribute::new(format!("{:?}_pfm_channel", party), val.channel.to_string()),
                ]
            }
            None => {
                vec![Attribute::new(format!("{:?}_pfm", party), "none")]
            }
        };

        let mut attributes = vec![
            Attribute::new(
                format!("{:?}_neutron_to_party_chain_port", party),
                "transfer".to_string(),
            ),
            Attribute::new(
                format!("{:?}_neutron_to_party_chain_channel", party),
                self.neutron_to_party_chain_channel.to_string(),
            ),
            Attribute::new(format!("{:?}_ibc_timeout", party), self.ibc_timeout),
        ];
        attributes.extend(pfm_attributes);

        attributes
    }
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        holder_address: Option<String>,
        note_address: Option<String>,
        ibc_config: Box<Option<IbcConfig>>,
        lp_config: Box<Option<LiquidityProvisionConfig>>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}
