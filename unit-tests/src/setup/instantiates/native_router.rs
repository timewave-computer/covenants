use std::collections::BTreeSet;

use cosmwasm_std::Addr;

use crate::setup::DENOM_ATOM_ON_NTRN;

pub struct NativeRouterInstantiate {
    pub msg: valence_native_router::msg::InstantiateMsg,
}

impl From<NativeRouterInstantiate> for valence_native_router::msg::InstantiateMsg {
    fn from(value: NativeRouterInstantiate) -> Self {
        value.msg
    }
}

impl NativeRouterInstantiate {
    pub fn new(
        privileged_accounts: Option<Vec<String>>,
        receiver_address: Addr,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self {
            msg: valence_native_router::msg::InstantiateMsg {
                privileged_accounts,
                receiver_address: receiver_address.to_string(),
                denoms,
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

    pub fn with_receiver_address(&mut self, addr: String) -> &mut Self {
        self.msg.receiver_address = addr;
        self
    }

    pub fn with_denoms(&mut self, denoms: BTreeSet<String>) -> &mut Self {
        self.msg.denoms = denoms;
        self
    }
}

impl NativeRouterInstantiate {
    pub fn default(privileged_accounts: Option<Vec<String>>, receiver_address: Addr) -> Self {
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        Self::new(privileged_accounts, receiver_address, denoms)
    }
}
