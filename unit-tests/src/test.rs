// use std::collections::BTreeMap;

// use cosmwasm_std::{
//     coin, coins, instantiate2_address, Addr, Api, CodeInfoResponse, Decimal, MemoryStorage,
// };
// use covenant_interchain_splitter::msg::SplitType;
// use covenant_utils::SplitConfig;
// use cw_multi_test::{
//     addons::{MockAddressGenerator, MockApiBech32},
//     App, BankKeeper, BasicAppBuilder, Executor, WasmKeeper,
// };
// use neutron_sdk::bindings::{
//     msg::{IbcFee, NeutronMsg},
//     query::NeutronQuery,
// };

// use sha2::{Digest, Sha256};

// use crate::setup::{
//     contracts::{
//         clock_contract, ibc_forwarder_contract, interchain_splitter_contract, stride_lser_contract,
//     },
//     custom_module::{NeutronKeeper, CHAIN_PREFIX},
//     DENOM_ATOM, DENOM_NTRN, FAUCET,
// };

// type CustomApp = App<
//     BankKeeper,
//     MockApiBech32,
//     MemoryStorage,
//     NeutronKeeper,
//     WasmKeeper<NeutronMsg, NeutronQuery>,
// >;

// pub fn get_addr(addr: &str) -> Addr {
//     cw_multi_test::addons::MockApiBech32::new(CHAIN_PREFIX).addr_make(addr)
// }

// // Do init2 and return the salt and the addr
// fn get_salt_and_addr(app: &CustomApp, code_id: u64, str: &str) -> (Vec<u8>, Addr) {
//     let mut hasher = Sha256::new();
//     hasher.update(str.to_string());
//     let salt = hasher.finalize().to_vec();

//     let canonical_creator = app
//         .api()
//         .addr_canonicalize(get_addr("admin").as_str())
//         .unwrap();
//     let CodeInfoResponse { checksum, .. } = app.wrap().query_wasm_code_info(code_id).unwrap();
//     let canonical_addr = instantiate2_address(&checksum, &canonical_creator, &salt).unwrap();
//     let addr = app.api().addr_humanize(&canonical_addr).unwrap();
//     (salt, addr)
// }

// ///  Test flow:
// /// 1. IBC forwarder expects have X `DENOM_ATOM` (to forward it)
// /// 2. IBC forwarder forward the X `DENOM_ATOM` to stride lser
// /// 3. we manually transfer stride funds from lser ICA to spilitter
// /// 4. on the splitter we expect the funds in the correct denom with IBC trace of the 2 chains
// #[test]
// fn test_stride() {
//     const NTRN_HUB_CHANNEL: &str = "channel-1";
//     const HUB_NTRN_CHANNEL: &str = "channel-10";

//     const NTRN_STRIDE_CHANNEL: &str = "channel-2";
//     const STRIDE_NTRN_CHANNEL: &str = "channel-20";

//     const HUB_STRIDE_CHANNEL: &str = "channel-3";
//     const STRIDE_HUB_CHANNEL: &str = "channel-30";

//     // This mocks our ls token denom on stride
//     let ATOM_ON_STRIDE_DENOM = format!("{STRIDE_HUB_CHANNEL}/{DENOM_ATOM}");
//     // This mocks our ls token denom on neutron
//     let STRIDE_LS_ON_NTRN_DENOM =
//         format!("{NTRN_STRIDE_CHANNEL}/{STRIDE_HUB_CHANNEL}/{DENOM_ATOM}");

//     let admin_addr = get_addr("admin");
//     let faucet_addr = get_addr(FAUCET);

//     let mut app = BasicAppBuilder::new_custom()
//         .with_custom(NeutronKeeper::new(CHAIN_PREFIX))
//         .with_api(MockApiBech32::new(CHAIN_PREFIX))
//         .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
//         .build(|router, _, storage| {
//             router
//                 .bank
//                 .init_balance(
//                     storage,
//                     &faucet_addr,
//                     vec![
//                         coin(1_000_000_000_000, DENOM_NTRN),
//                         coin(1_000_000_000_000, DENOM_ATOM),
//                     ],
//                 )
//                 .unwrap();

//             // add channels
//             // Local channels from ntrn to other chains
//             router
//                 .custom
//                 .add_local_channel(storage, NTRN_HUB_CHANNEL, HUB_NTRN_CHANNEL)
//                 .unwrap();
//             router
//                 .custom
//                 .add_local_channel(storage, NTRN_STRIDE_CHANNEL, STRIDE_NTRN_CHANNEL)
//                 .unwrap();

//             // Remote channels from other chains to other chain
//             router
//                 .custom
//                 .add_remote_channel(storage, HUB_STRIDE_CHANNEL, STRIDE_HUB_CHANNEL)
//                 .unwrap();
//         });

//     // Upload contracts
//     let clock_code_id = app.store_code(clock_contract());
//     let ibc_forwarder_code_id = app.store_code(ibc_forwarder_contract());
//     let stride_lser_code_id = app.store_code(stride_lser_contract());
//     let splitter_code_id = app.store_code(interchain_splitter_contract());

//     // Get init 2 data
//     let (forwarder_salt, forwarder_addr) =
//         get_salt_and_addr(&app, ibc_forwarder_code_id, "ibc_forwarder");
//     let (stride_salt, stride_addr) = get_salt_and_addr(&app, stride_lser_code_id, "stride");
//     let (splitter_salt, splitter_addr) = get_salt_and_addr(&app, splitter_code_id, "splitter");

//     // init clock
//     let clock_addr = app
//         .instantiate_contract(
//             clock_code_id,
//             admin_addr.clone(),
//             &covenant_clock::msg::InstantiateMsg {
//                 tick_max_gas: None,
//                 whitelist: vec![
//                     forwarder_addr.to_string(),
//                     splitter_addr.to_string(),
//                     stride_addr.to_string(),
//                 ],
//             },
//             &[],
//             "clock",
//             Some(admin_addr.to_string()),
//         )
//         .unwrap();

//     // init forwarder
//     app.instantiate2_contract(
//         ibc_forwarder_code_id,
//         admin_addr.clone(),
//         &covenant_ibc_forwarder::msg::InstantiateMsg {
//             clock_address: clock_addr.to_string(),
//             next_contract: stride_addr.to_string(),
//             remote_chain_connection_id: "conn-1".to_string(),
//             remote_chain_channel_id: HUB_STRIDE_CHANNEL.to_string(),
//             denom: DENOM_ATOM.to_string(),
//             amount: 1000_u128.into(),
//             ibc_fee: IbcFee {
//                 recv_fee: vec![],
//                 ack_fee: coins(100_u128, DENOM_NTRN),
//                 timeout_fee: coins(100_u128, DENOM_NTRN),
//             },
//             ibc_transfer_timeout: 1000_u64.into(),
//             ica_timeout: 1000_u64.into(),
//         },
//         &[],
//         "forwarder",
//         Some(admin_addr.to_string()),
//         forwarder_salt,
//     )
//     .unwrap();

//     // init stride
//     app.instantiate2_contract(
//         stride_lser_code_id,
//         admin_addr.clone(),
//         &covenant_stride_liquid_staker::msg::InstantiateMsg {
//             clock_address: clock_addr.to_string(),
//             stride_neutron_ibc_transfer_channel_id: STRIDE_NTRN_CHANNEL.to_string(),
//             neutron_stride_ibc_connection_id: "conn-1".to_string(),
//             next_contract: splitter_addr.to_string(),
//             ls_denom: ATOM_ON_STRIDE_DENOM.clone(),
//             ibc_fee: IbcFee {
//                 recv_fee: vec![],
//                 ack_fee: coins(100_u128, DENOM_NTRN),
//                 timeout_fee: coins(100_u128, DENOM_NTRN),
//             },
//             ibc_transfer_timeout: 1000_u64.into(),
//             ica_timeout: 1000_u64.into(),
//         },
//         &[],
//         "lser",
//         Some(admin_addr.to_string()),
//         stride_salt,
//     )
//     .unwrap();

//     // init splitter
//     let mut splits: BTreeMap<String, Decimal> = BTreeMap::new();
//     splits.insert("addr".to_string(), Decimal::bps(5000));
//     splits.insert("addr2".to_string(), Decimal::bps(5000));

//     app.instantiate2_contract(
//         splitter_code_id,
//         admin_addr.clone(),
//         &covenant_interchain_splitter::msg::InstantiateMsg {
//             clock_address: clock_addr.to_string(),
//             splits: vec![(
//                 "denom".to_string(),
//                 SplitType::Custom(SplitConfig { receivers: splits }),
//             )],
//             fallback_split: None,
//         },
//         &[],
//         "splitter",
//         Some(admin_addr.to_string()),
//         splitter_salt,
//     )
//     .unwrap();

//     // fund contracts with neutron to do IBC stuff
//     app.send_tokens(
//         faucet_addr.clone(),
//         forwarder_addr.clone(),
//         &coins(1_100_000, DENOM_NTRN),
//     )
//     .unwrap();
//     app.send_tokens(
//         faucet_addr.clone(),
//         stride_addr.clone(),
//         &coins(1_100_000, DENOM_NTRN),
//     )
//     .unwrap();
//     app.send_tokens(
//         faucet_addr.clone(),
//         splitter_addr.clone(),
//         &coins(1_100_000, DENOM_NTRN),
//     )
//     .unwrap();

//     // Verify we don't have ICA yet
//     app.wrap()
//         .query_wasm_smart::<Option<String>>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::IcaAddress {},
//         )
//         .unwrap_err();

//     // Do tick on forwarder to create ICA
//     app.execute_contract(
//         clock_addr.clone(),
//         forwarder_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     let forwarder_ica = app
//         .wrap()
//         .query_wasm_smart::<Option<String>>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::IcaAddress {},
//         )
//         .unwrap()
//         .unwrap();
//     let forwarder_ica = Addr::unchecked(forwarder_ica);

//     // Fund the forwarder ICA with the wanted amount (1_000 ATOM)
//     app.send_tokens(
//         faucet_addr,
//         forwarder_ica.clone(),
//         &coins(1_000, DENOM_ATOM),
//     )
//     .unwrap();

//     // Do tick on lser to create ICA on stride
//     app.execute_contract(
//         clock_addr.clone(),
//         stride_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     let stride_ica = app
//         .wrap()
//         .query_wasm_smart::<Option<String>>(
//             stride_addr.clone(),
//             &covenant_stride_liquid_staker::msg::QueryMsg::IcaAddress {},
//         )
//         .unwrap()
//         .unwrap();
//     let stride_ica = Addr::unchecked(stride_ica);

//     // Do tick on forwarder to send funds to the stride ICA ("results in ls tokens")
//     app.execute_contract(
//         clock_addr.clone(),
//         forwarder_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     // Verify stride ICA holds the correct denom
//     let stride_ica_balance = app
//         .wrap()
//         .query_balance(stride_ica.clone(), ATOM_ON_STRIDE_DENOM.clone())
//         .unwrap();

//     assert_eq!(
//         stride_ica_balance,
//         coin(1_000_u128, ATOM_ON_STRIDE_DENOM.clone())
//     );

//     // Transfer from stride ICA to splitter
//     app.execute_contract(
//         admin_addr.clone(),
//         stride_addr,
//         &covenant_stride_liquid_staker::msg::ExecuteMsg::Transfer {
//             amount: 1_000_u128.into(),
//         },
//         &[],
//     )
//     .unwrap();

//     // Verify we got the funds on the splitter, with the correct denom
//     let splitter_balance = app
//         .wrap()
//         .query_balance(splitter_addr.clone(), STRIDE_LS_ON_NTRN_DENOM.clone())
//         .unwrap();
//     assert_eq!(splitter_balance, coin(1_000_u128, STRIDE_LS_ON_NTRN_DENOM));
// }

// #[test]
// fn test_timeout() {
//     let admin_addr = get_addr("admin");

//     let mut app = BasicAppBuilder::new_custom()
//         .with_custom(NeutronKeeper::new(CHAIN_PREFIX))
//         .with_api(MockApiBech32::new(CHAIN_PREFIX))
//         .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
//         .build(|_, _, _| {});

//     let clock_code_id = app.store_code(clock_contract());
//     let ibc_forwarder_code_id = app.store_code(ibc_forwarder_contract());
//     let stride_lser_code_id = app.store_code(stride_lser_contract());

//     // Get init 2 data
//     let (forwarder_salt, forwarder_addr) =
//         get_salt_and_addr(&app, ibc_forwarder_code_id, "ibc_forwarder");
//     let (stride_salt, stride_addr) = get_salt_and_addr(&app, stride_lser_code_id, "stride");

//     // fund contracts with ntrn for ibc stuff
//     app.init_modules(|r, _, s| {
//         r.bank
//             .init_balance(s, &forwarder_addr, vec![coin(1_000_000_000, DENOM_NTRN)])
//             .unwrap();

//         r.bank
//             .init_balance(s, &stride_addr, vec![coin(1_000_000_000, DENOM_NTRN)])
//             .unwrap();
//     });

//     let clock_addr = app
//         .instantiate_contract(
//             clock_code_id,
//             admin_addr.clone(),
//             &covenant_clock::msg::InstantiateMsg {
//                 tick_max_gas: None,
//                 whitelist: vec![forwarder_addr.to_string(), stride_addr.to_string()],
//             },
//             &[],
//             "clock",
//             Some(admin_addr.to_string()),
//         )
//         .unwrap();

//     app.instantiate2_contract(
//         ibc_forwarder_code_id,
//         admin_addr.clone(),
//         &covenant_ibc_forwarder::msg::InstantiateMsg {
//             clock_address: clock_addr.to_string(),
//             next_contract: stride_addr.to_string(),
//             remote_chain_connection_id: "conn-1".to_string(),
//             remote_chain_channel_id: "channel-1".to_string(),
//             denom: DENOM_ATOM.to_string(),
//             amount: 1000_u128.into(),
//             ibc_fee: IbcFee {
//                 recv_fee: vec![],
//                 ack_fee: coins(100_u128, DENOM_NTRN),
//                 timeout_fee: coins(100_u128, DENOM_NTRN),
//             },
//             ibc_transfer_timeout: 1000_u64.into(),
//             ica_timeout: 1000_u64.into(),
//         },
//         &[],
//         "forwarder",
//         Some(admin_addr.to_string()),
//         forwarder_salt,
//     )
//     .unwrap();

//     app.instantiate2_contract(
//         stride_lser_code_id,
//         admin_addr.clone(),
//         &covenant_stride_liquid_staker::msg::InstantiateMsg {
//             clock_address: clock_addr.to_string(),
//             stride_neutron_ibc_transfer_channel_id: "channel-2".to_string(),
//             neutron_stride_ibc_connection_id: "conn-1".to_string(),
//             next_contract: forwarder_addr.to_string(),
//             ls_denom: "some_denom".to_string(),
//             ibc_fee: IbcFee {
//                 recv_fee: vec![],
//                 ack_fee: coins(100_u128, DENOM_NTRN),
//                 timeout_fee: coins(100_u128, DENOM_NTRN),
//             },
//             ibc_transfer_timeout: 1000_u64.into(),
//             ica_timeout: 1000_u64.into(),
//         },
//         &[],
//         "lser",
//         Some(admin_addr.to_string()),
//         stride_salt,
//     )
//     .unwrap();

//     let forwarder_state = app
//         .wrap()
//         .query_wasm_smart::<covenant_ibc_forwarder::msg::ContractState>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::ContractState {},
//         )
//         .unwrap();

//     assert_eq!(
//         forwarder_state,
//         covenant_ibc_forwarder::msg::ContractState::Instantiated
//     );

//     // Do tick on forwarder to create ICA
//     app.execute_contract(
//         clock_addr.clone(),
//         forwarder_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     // Do tick on stride lser to create ICA
//     app.execute_contract(
//         clock_addr.clone(),
//         stride_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     let forwarder_state = app
//         .wrap()
//         .query_wasm_smart::<covenant_ibc_forwarder::msg::ContractState>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::ContractState {},
//         )
//         .unwrap();

//     assert_eq!(
//         forwarder_state,
//         covenant_ibc_forwarder::msg::ContractState::IcaCreated
//     );

//     // Set timeout to be called on the next tick
//     app.init_modules(|r, _, _| {
//         r.custom.set_timeout(true);
//     });

//     // Do tick on forwarder to trigger a timeout response
//     app.execute_contract(
//         clock_addr.clone(),
//         forwarder_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     // The state now should be set back to Instantiated because we had a timeout
//     let forwarder_state = app
//         .wrap()
//         .query_wasm_smart::<covenant_ibc_forwarder::msg::ContractState>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::ContractState {},
//         )
//         .unwrap();

//     assert_eq!(
//         forwarder_state,
//         covenant_ibc_forwarder::msg::ContractState::Instantiated
//     );

//     // Turn off the timeout flag
//     app.init_modules(|r, _, _| {
//         r.custom.set_timeout(false);
//     });

//     // Do another tick to create a new ICA
//     app.execute_contract(
//         clock_addr.clone(),
//         forwarder_addr.clone(),
//         &covenant_clock::msg::ExecuteMsg::Tick {},
//         &[],
//     )
//     .unwrap();

//     // The state should be back to IcaCreated now again
//     let forwarder_state = app
//         .wrap()
//         .query_wasm_smart::<covenant_ibc_forwarder::msg::ContractState>(
//             forwarder_addr.clone(),
//             &covenant_ibc_forwarder::msg::QueryMsg::ContractState {},
//         )
//         .unwrap();

//     assert_eq!(
//         forwarder_state,
//         covenant_ibc_forwarder::msg::ContractState::IcaCreated
//     );
// }
