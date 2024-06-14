use std::collections::BTreeSet;

use cosmwasm_std::Addr;
use covenant_utils::op_mode::ContractOperationModeConfig;

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
        op_mode_cfg: ContractOperationModeConfig,
        receiver_address: Addr,
        denoms: BTreeSet<String>,
    ) -> Self {
        Self {
            msg: valence_native_router::msg::InstantiateMsg {
                op_mode_cfg,
                receiver_address: receiver_address.to_string(),
                denoms,
            },
        }
    }

    pub fn with_op_mode(&mut self, op_mode: ContractOperationModeConfig) -> &mut Self {
        self.msg.op_mode_cfg = op_mode;
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
    pub fn default(op_mode: ContractOperationModeConfig, receiver_address: Addr) -> Self {
        let denoms = BTreeSet::from_iter(vec![DENOM_ATOM_ON_NTRN.to_string()]);

        Self::new(op_mode, receiver_address, denoms)
    }
}
