use cosmwasm_std::{Uint128, Uint64};

use crate::setup::{DENOM_ATOM_ON_NTRN, NTRN_HUB_CHANNEL};

pub struct IbcForwarderInstantiate {
    pub msg: valence_ibc_forwarder::msg::InstantiateMsg,
}

impl From<IbcForwarderInstantiate> for valence_ibc_forwarder::msg::InstantiateMsg {
    fn from(value: IbcForwarderInstantiate) -> Self {
        value.msg
    }
}

impl IbcForwarderInstantiate {
    pub fn new(
        privileged_accounts: Option<Vec<String>>,
        next_contract: String,
        remote_chain_connection_id: String,
        remote_chain_channel_id: String,
        denom: String,
        amount: Uint128,
        ibc_transfer_timeout: Uint64,
        ica_timeout: Uint64,
        fallback_address: Option<String>,
    ) -> Self {
        Self {
            msg: valence_ibc_forwarder::msg::InstantiateMsg {
                privileged_accounts,
                next_contract,
                remote_chain_connection_id,
                remote_chain_channel_id,
                denom,
                amount,
                ibc_transfer_timeout,
                ica_timeout,
                fallback_address,
            },
        }
    }

    pub fn with_privileged_accounts(
        &mut self,
        privileged_accounts: Option<Vec<String>>,
    ) -> &mut Self {
        self.msg.privileged_accounts = privileged_accounts;
        self
    }

    pub fn with_next_contract(&mut self, addr: String) -> &mut Self {
        self.msg.next_contract = addr;
        self
    }

    pub fn with_remote_chain_connection_id(&mut self, addr: String) -> &mut Self {
        self.msg.remote_chain_connection_id = addr;
        self
    }

    pub fn with_remote_chain_channel_id(&mut self, addr: String) -> &mut Self {
        self.msg.remote_chain_channel_id = addr;
        self
    }

    pub fn with_denom(&mut self, addr: String) -> &mut Self {
        self.msg.denom = addr;
        self
    }

    pub fn with_amount(&mut self, addr: Uint128) -> &mut Self {
        self.msg.amount = addr;
        self
    }

    pub fn with_fallback_address(&mut self, addr: String) -> &mut Self {
        self.msg.fallback_address = Some(addr);
        self
    }

    pub fn with_ibc_transfer_timeout(&mut self, addr: Uint64) -> &mut Self {
        self.msg.ibc_transfer_timeout = addr;
        self
    }

    pub fn with_ica_timeout(&mut self, addr: Uint64) -> &mut Self {
        self.msg.ica_timeout = addr;
        self
    }
}

impl IbcForwarderInstantiate {
    pub fn default(
        privileged_accounts: Option<Vec<String>>,
        next_contract: String,
        fallback_address: Option<String>,
    ) -> Self {
        Self {
            msg: valence_ibc_forwarder::msg::InstantiateMsg {
                privileged_accounts,
                next_contract,
                remote_chain_connection_id: "connection-todo".to_string(),
                remote_chain_channel_id: NTRN_HUB_CHANNEL.1.to_string(),
                denom: DENOM_ATOM_ON_NTRN.to_string(),
                amount: Uint128::new(100_000),
                ica_timeout: Uint64::from(100u64),
                ibc_transfer_timeout: Uint64::from(100u64),
                fallback_address,
            },
        }
    }
}
