// use cosmwasm_std::{Addr, Empty, Uint128, Uint64};
// use cw_multi_test::{App, Contract, ContractWrapper};

// use crate::msg::{InstantiateMsg, PresetIbcFee, Timeouts};

// pub const _CREATOR_ADDR: &str = "admin";
// pub const _TODO: &str = "replace";

// fn _covenant_clock() -> Box<dyn Contract<Empty>> {
//     Box::new(
//         ContractWrapper::new(
//             covenant_clock::contract::execute,
//             covenant_clock::contract::instantiate,
//             covenant_clock::contract::query,
//         )
//         .with_reply(covenant_clock::contract::reply)
//         .with_migrate(covenant_clock::contract::migrate),
//     )
// }

// pub(crate) struct Suite {
//     pub app: App,
//     pub covenant_address: Addr,
// }

// pub(crate) struct SuiteBuilder {
//     pub instantiate: InstantiateMsg,
// }

// impl Default for SuiteBuilder {
//     fn default() -> Self {
//         Self {
//             instantiate: InstantiateMsg {
//                 label: "swap-covenant".to_string(),
//                 preset_ibc_fee: PresetIbcFee {
//                     ack_fee: Uint128::new(1000),
//                     timeout_fee: Uint128::new(1000),
//                 },
//                 timeouts: Timeouts {
//                     ica_timeout: Uint64::new(50),
//                     ibc_transfer_timeout: Uint64::new(50),
//                 },
//                 contract_codes: todo!(),
//                 clock_tick_max_gas: todo!(),
//                 lockup_config: todo!(),
//                 covenant_terms: todo!(),
//                 party_a_config: todo!(),
//                 party_b_config: todo!(),
//                 splits: todo!(),
//                 fallback_split: todo!(),
//             },
//         }
//     }
// }

// impl SuiteBuilder {
//     pub fn build(self) -> Suite {
//         let app = App::default();
//         Suite {
//             app,
//             covenant_address: todo!(),
//         }
//     }
// }

// // assertion helpers
// impl Suite {}

// // queries
// impl Suite {}
