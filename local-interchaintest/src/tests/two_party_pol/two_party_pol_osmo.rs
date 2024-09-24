use std::{collections::BTreeMap, env, path::Path, str::FromStr, thread, time::Duration};

use anyhow::Error;
use cosmwasm_std::{Coin, Decimal, Uint128, Uint64};
use covenant_utils::{
    op_mode::ContractOperationModeConfig, split::SplitConfig, ForwardMetadata,
    InterchainCovenantParty, PacketForwardMiddlewareConfig, PoolPriceConfig, SingleSideLpLimits,
};
use cw_utils::Expiration;
use localic_std::{
    modules::{
        bank::{get_balance, send},
        cosmwasm::{contract_execute, contract_instantiate, contract_query, CosmWasm},
    },
    node::Chain,
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, GAIA_CHAIN_NAME, NEUTRON_CHAIN_ADMIN_ADDR,
    NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME, TRANSFER_PORT,
};
use log::info;
use valence_covenant_two_party_pol::msg::{CovenantContractCodeIds, CovenantPartyConfig, Timeouts};
use valence_osmo_liquid_pooler::msg::{OsmosisLiquidPoolerConfig, PartyChainInfo, PartyDenomInfo};
use valence_two_party_pol_holder::msg::CovenantType;

use crate::{
    helpers::{
        common::{query_contract_state, tick},
        constants::{
            ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC1_ADDRESS_OSMO, ACC2_ADDRESS_GAIA,
            ACC2_ADDRESS_NEUTRON, ACC2_ADDRESS_OSMO, ACC_1_KEY, ACC_2_KEY, EXECUTE_FLAGS,
            LOCAL_CODE_ID_CACHE_PATH, OSMOSIS_FEES, POLYTONE_PATH, VALENCE_PATH,
        },
        covenant::Covenant,
    },
    send_non_native_balances,
};

pub fn test_two_party_pol_osmo(test_ctx: &mut TestContext) -> Result<(), Error> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    // Store only the ones we need for the test
    let polytone_note_path = format!("{POLYTONE_PATH}/polytone_note.wasm");
    let polytone_voice_path = format!("{POLYTONE_PATH}/polytone_voice.wasm");
    let polytone_proxy_path = format!("{POLYTONE_PATH}/polytone_proxy.wasm");
    let osmosis_outpost_path = format!("{VALENCE_PATH}/valence_outpost_osmo_liquid_pooler.wasm");

    let current_dir = env::current_dir()?;

    let mut cw = CosmWasm::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );
    let polytone_note_code_id = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(polytone_note_path).as_path())?,
        )
        .unwrap();

    let mut cw = CosmWasm::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
    );

    let polytone_voice_code_id = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(polytone_voice_path).as_path())?,
        )
        .unwrap();

    let polytone_proxy_code_id = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(polytone_proxy_path).as_path())?,
        )
        .unwrap();

    let osmosis_outpost_code_id = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(osmosis_outpost_path).as_path())?,
        )
        .unwrap();

    info!("Starting two party POL osmo tests...");
    info!("Create and add liquidity to ATOM-OSMO pool");

    let osmosis_admin_acc = test_ctx.get_admin_addr().src(OSMOSIS_CHAIN_NAME).get();
    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN_NAME).get();
    let neutron_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN_NAME).get();
    let osmo_denom = test_ctx.get_native_denom().src(OSMOSIS_CHAIN_NAME).get();
    let osmo_on_neutron = test_ctx
        .get_ibc_denom()
        .base_denom(osmo_denom.to_owned())
        .src(OSMOSIS_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();
    let osmo_on_gaia = test_ctx
        .get_ibc_denom()
        .base_denom(osmo_denom.to_owned())
        .src(OSMOSIS_CHAIN_NAME)
        .dest(GAIA_CHAIN_NAME)
        .get();
    let atom_on_neutron = test_ctx
        .get_ibc_denom()
        .base_denom(atom_denom.to_owned())
        .src(GAIA_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();
    let atom_on_osmosis = test_ctx
        .get_ibc_denom()
        .base_denom(atom_denom.to_owned())
        .src(GAIA_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    // Send some ATOM to Osmosis to feed the pool
    let uatom_liquidity = 40_000_000_000;
    let uosmo_liquidity = 330_000_000_000;
    loop {
        test_ctx
            .build_tx_transfer()
            .with_chain_name(GAIA_CHAIN_NAME)
            .with_amount(uatom_liquidity)
            .with_recipient(&osmosis_admin_acc)
            .with_denom(&atom_denom)
            .send()
            .unwrap();

        info!("Waiting to receive ATOM IBC transfer...");
        thread::sleep(Duration::from_secs(5));
        let balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmosis_admin_acc,
        );
        if balance
            .iter()
            .any(|c| c.denom == atom_on_osmosis && c.amount >= Uint128::new(uatom_liquidity))
        {
            break;
        }
    }

    test_ctx
        .build_tx_create_osmo_pool()
        .with_weight(&atom_on_osmosis, 50)
        .with_weight(&osmo_denom, 50)
        .with_initial_deposit(&atom_on_osmosis, uatom_liquidity as u64)
        .with_initial_deposit(&osmo_denom, uosmo_liquidity)
        .with_swap_fee(Decimal::from_str("0.003").unwrap())
        .send()?;

    let pool_id = test_ctx
        .get_osmo_pool()
        .denoms(osmo_denom.to_owned(), atom_on_osmosis.to_owned())
        .get_u64();

    info!("Instantiate Osmosis outpost contract");
    let osmo_liquid_pooler_instantiate_msg =
        valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg {};
    let osmosis_outpost_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        osmosis_outpost_code_id,
        &serde_json::to_string(&osmo_liquid_pooler_instantiate_msg).unwrap(),
        "osmosis-outpost",
        None,
        OSMOSIS_FEES,
    )
    .unwrap();

    info!("Instantiate Polytone Note on Neutron");
    let polytone_note_instantiate_msg = polytone_note::msg::InstantiateMsg {
        pair: None,
        block_max_gas: Uint64::new(3010000),
    };
    let polytone_note_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        polytone_note_code_id,
        &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
        "polytone-note",
        None,
        "",
    )
    .unwrap();

    info!("Instantiate Polytone Voice on Osmosis");
    let polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_code_id),
        block_max_gas: Uint64::new(3010000),
    };

    let polytone_voice_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        polytone_voice_code_id,
        &serde_json::to_string(&polytone_voice_instantiate_msg).unwrap(),
        "polytone-voice",
        None,
        OSMOSIS_FEES,
    )
    .unwrap();

    info!("Create polytone channel");
    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );

    CosmWasm::new_from_existing(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        None,
        None,
        Some(polytone_note_contract.address.clone()),
    )
    .create_wasm_connection(
        &relayer,
        "neutron-osmosis",
        &CosmWasm::new_from_existing(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            None,
            None,
            Some(polytone_voice_contract.address.clone()),
        ),
        "unordered",
        "polytone-1",
    )
    .unwrap();

    let valence_ibc_forwarder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_ibc_forwarder")
        .unwrap();

    let valence_two_party_pol_holder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_two_party_pol_holder")
        .unwrap();

    let valence_clock_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_clock")
        .unwrap();

    let valence_interchain_router_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_interchain_router")
        .unwrap();

    let valence_native_router_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_native_router")
        .unwrap();

    let valence_liquid_pooler_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_osmo_liquid_pooler")
        .unwrap();

    let valence_covenant_two_party_pol_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_covenant_two_party_pol")
        .unwrap();

    let uatom_contribution_amount = 500_000_000;
    let uosmo_contribution_amount = 5_000_000_000;

    let target = "Two party Osmosis POL withdraw pre-lp tokens";
    info!(target: target,"Starting Two party Osmosis POL withdraw pre-lp tokens test...");
    let current_block_height = Chain::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    )
    .get_height();

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "covenant-osmo".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 210),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC1_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::from([(
                osmo_on_neutron.clone(),
                PacketForwardMiddlewareConfig {
                    local_to_hop_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    hop_to_destination_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(OSMOSIS_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    hop_chain_receiver_address: ACC1_ADDRESS_OSMO.to_string(),
                },
            )]),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(OSMOSIS_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(OSMOSIS_CHAIN_NAME)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::from([(
                atom_on_neutron.clone(),
                PacketForwardMiddlewareConfig {
                    local_to_hop_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    hop_to_destination_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(GAIA_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    hop_chain_receiver_address: ACC2_ADDRESS_GAIA.to_string(),
                },
            )]),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Share,
        ragequit_config: None,
        deposit_deadline: Expiration::AtHeight(current_block_height + 200),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.51").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
        ]),
        fallback_split: None,
        emergency_committee: Some(NEUTRON_CHAIN_ADMIN_ADDR.to_string()),
        liquid_pooler_config: valence_covenant_two_party_pol::msg::LiquidPoolerConfig::Osmosis(
            Box::new(OsmosisLiquidPoolerConfig {
                note_address: polytone_note_contract.address.clone(),
                pool_id: Uint64::new(pool_id),
                osmo_ibc_timeout: Uint64::new(300),
                osmo_outpost: osmosis_outpost_contract.address.clone(),
                party_1_chain_info: PartyChainInfo {
                    neutron_to_party_chain_channel: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    party_chain_to_neutron_channel: test_ctx
                        .get_transfer_channels()
                        .src(GAIA_CHAIN_NAME)
                        .dest(NEUTRON_CHAIN_NAME)
                        .get(),
                    outwards_pfm: Some(ForwardMetadata {
                        receiver: ACC1_ADDRESS_GAIA.to_string(),
                        port: TRANSFER_PORT.to_string(),
                        channel: test_ctx
                            .get_transfer_channels()
                            .src(GAIA_CHAIN_NAME)
                            .dest(OSMOSIS_CHAIN_NAME)
                            .get(),
                    }),
                    inwards_pfm: Some(ForwardMetadata {
                        receiver: ACC1_ADDRESS_GAIA.to_string(),
                        port: TRANSFER_PORT.to_string(),
                        channel: test_ctx
                            .get_transfer_channels()
                            .src(OSMOSIS_CHAIN_NAME)
                            .dest(GAIA_CHAIN_NAME)
                            .get(),
                    }),
                    ibc_timeout: Uint64::new(300),
                },
                party_2_chain_info: PartyChainInfo {
                    neutron_to_party_chain_channel: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    party_chain_to_neutron_channel: test_ctx
                        .get_transfer_channels()
                        .src(OSMOSIS_CHAIN_NAME)
                        .dest(NEUTRON_CHAIN_NAME)
                        .get(),
                    outwards_pfm: None,
                    inwards_pfm: None,
                    ibc_timeout: Uint64::new(300),
                },
                lp_token_denom: format!("gamm/pool/{pool_id}"),
                osmo_to_neutron_channel_id: test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                party_1_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin {
                        denom: atom_on_osmosis.clone(),
                        amount: Uint128::new(uatom_contribution_amount),
                    },
                    local_denom: atom_on_neutron.clone(),
                },
                party_2_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin {
                        denom: osmo_denom.clone(),
                        amount: Uint128::new(uosmo_contribution_amount),
                    },
                    local_denom: osmo_on_neutron.clone(),
                },
                funding_duration: cw_utils::Duration::Time(400),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(10000),
                    asset_b_limit: Uint128::new(975000004),
                },
            }),
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "covenant-osmo",
        None,
        "",
    )
    .unwrap();
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        contract_address: &covenant_contract.address,
    };

    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
        party_a_ibc_forwarder_address.clone(),
        party_b_ibc_forwarder_address.clone(),
    ];

    for address in &addresses {
        send(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            address,
            &[Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000000000),
            }],
            &Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000),
            },
        )
        .unwrap();
    }

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
        let forwarder_a_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_a_ibc_forwarder_address,
        );
        let forwarder_b_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_b_ibc_forwarder_address,
        );
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        DEFAULT_KEY,
        &party_a_deposit_address,
        &[Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(uatom_contribution_amount),
        }],
        &Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();
    send(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        let holder_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    loop {
        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        if liquid_pooler_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && liquid_pooler_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            break;
        }
        {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until Liquid Pooler Proxy is created...");
    let proxy_address;
    loop {
        let liquid_pooler_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler state: {:?}", liquid_pooler_state);
        if liquid_pooler_state == "proxy_created" {
            let query_response = contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &liquid_pooler_address,
                &serde_json::to_string(&valence_osmo_liquid_pooler::msg::QueryMsg::ProxyAddress {})
                    .unwrap(),
            );

            proxy_address = query_response["data"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            info!(target: target,"Proxy address: {}", proxy_address);
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until Proxy is funded...");
    loop {
        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        if proxy_balance
            .iter()
            .any(|c| c.denom == atom_on_osmosis || c.denom == osmo_denom)
        {
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Perform emergency withdrawal...");
    let liquid_pooler_state = query_contract_state(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &liquid_pooler_address,
    );
    info!(target: target,"Liquid Pooler state: {:?}", liquid_pooler_state);
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &holder_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_two_party_pol_holder::msg::ExecuteMsg::EmergencyWithdraw {},
        )
        .unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();

    loop {
        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler balance: {:?}", liquid_pooler_balance);

        let party_a_router_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_a_router_address,
        );
        info!(target: target,"Party A router balance: {:?}", party_a_router_balance);

        let party_b_router_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_b_router_address,
        );
        info!(target: target,"Party B router balance: {:?}", party_b_router_balance);

        if party_a_router_balance
            .iter()
            .any(|c| c.denom == atom_on_neutron || c.denom == osmo_on_neutron)
            || party_b_router_balance
                .iter()
                .any(|c| c.denom == atom_on_neutron || c.denom == osmo_on_neutron)
        {
            info!(target: target,"Withdraw successful!");
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until parties get the funds");
    loop {
        let hub_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            ACC1_ADDRESS_GAIA,
        );
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        let osmo_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            ACC2_ADDRESS_OSMO,
        );
        info!(target: target,"Osmo receiver balances: {:?}", osmo_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == osmo_on_gaia)
            && osmo_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_osmosis)
        {
            info!(target: target, "Parties received the funds!");
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    // Send the balances back so we have a fresh start for the next test
    send_non_native_balances(
        test_ctx,
        GAIA_CHAIN_NAME,
        ACC_1_KEY,
        ACC1_ADDRESS_GAIA,
        NEUTRON_CHAIN_ADMIN_ADDR,
        &atom_denom,
    );

    send_non_native_balances(
        test_ctx,
        OSMOSIS_CHAIN_NAME,
        ACC_2_KEY,
        ACC2_ADDRESS_OSMO,
        NEUTRON_CHAIN_ADMIN_ADDR,
        &osmo_denom,
    );

    let target = "Two party Osmosis POL full path";
    info!(target: target,"Two party Osmosis POL full path test...");
    let current_block_height = Chain::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    )
    .get_height();

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "covenant-osmo".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 300),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC1_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::from([(
                osmo_on_neutron.clone(),
                PacketForwardMiddlewareConfig {
                    local_to_hop_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    hop_to_destination_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(OSMOSIS_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    hop_chain_receiver_address: ACC1_ADDRESS_OSMO.to_string(),
                },
            )]),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(OSMOSIS_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(OSMOSIS_CHAIN_NAME)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::from([(
                atom_on_neutron.clone(),
                PacketForwardMiddlewareConfig {
                    local_to_hop_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    hop_to_destination_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(GAIA_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    hop_chain_receiver_address: ACC2_ADDRESS_GAIA.to_string(),
                },
            )]),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Share,
        ragequit_config: None,
        deposit_deadline: Expiration::AtHeight(current_block_height + 250),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
        ]),
        fallback_split: None,
        emergency_committee: Some(NEUTRON_CHAIN_ADMIN_ADDR.to_string()),
        liquid_pooler_config: valence_covenant_two_party_pol::msg::LiquidPoolerConfig::Osmosis(
            Box::new(OsmosisLiquidPoolerConfig {
                note_address: polytone_note_contract.address,
                pool_id: Uint64::new(pool_id),
                osmo_ibc_timeout: Uint64::new(300),
                osmo_outpost: osmosis_outpost_contract.address.clone(),
                party_1_chain_info: PartyChainInfo {
                    neutron_to_party_chain_channel: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    party_chain_to_neutron_channel: test_ctx
                        .get_transfer_channels()
                        .src(GAIA_CHAIN_NAME)
                        .dest(NEUTRON_CHAIN_NAME)
                        .get(),
                    outwards_pfm: Some(ForwardMetadata {
                        receiver: ACC1_ADDRESS_GAIA.to_string(),
                        port: TRANSFER_PORT.to_string(),
                        channel: test_ctx
                            .get_transfer_channels()
                            .src(GAIA_CHAIN_NAME)
                            .dest(OSMOSIS_CHAIN_NAME)
                            .get(),
                    }),
                    inwards_pfm: Some(ForwardMetadata {
                        receiver: ACC1_ADDRESS_GAIA.to_string(),
                        port: TRANSFER_PORT.to_string(),
                        channel: test_ctx
                            .get_transfer_channels()
                            .src(OSMOSIS_CHAIN_NAME)
                            .dest(GAIA_CHAIN_NAME)
                            .get(),
                    }),
                    ibc_timeout: Uint64::new(300),
                },
                party_2_chain_info: PartyChainInfo {
                    neutron_to_party_chain_channel: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(OSMOSIS_CHAIN_NAME)
                        .get(),
                    party_chain_to_neutron_channel: test_ctx
                        .get_transfer_channels()
                        .src(OSMOSIS_CHAIN_NAME)
                        .dest(NEUTRON_CHAIN_NAME)
                        .get(),
                    outwards_pfm: None,
                    inwards_pfm: None,
                    ibc_timeout: Uint64::new(300),
                },
                lp_token_denom: format!("gamm/pool/{pool_id}"),
                osmo_to_neutron_channel_id: test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                party_1_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin {
                        denom: atom_on_osmosis.clone(),
                        amount: Uint128::new(uatom_contribution_amount),
                    },
                    local_denom: atom_on_neutron.clone(),
                },
                party_2_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin {
                        denom: osmo_denom.clone(),
                        amount: Uint128::new(uosmo_contribution_amount),
                    },
                    local_denom: osmo_on_neutron.clone(),
                },
                funding_duration: cw_utils::Duration::Time(400),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(10000),
                    asset_b_limit: Uint128::new(975000004),
                },
            }),
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "covenant-osmo",
        None,
        "",
    )
    .unwrap();
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        contract_address: &covenant_contract.address,
    };

    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
        party_a_ibc_forwarder_address.clone(),
        party_b_ibc_forwarder_address.clone(),
    ];

    for address in &addresses {
        send(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            address,
            &[Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000000000),
            }],
            &Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000),
            },
        )
        .unwrap();
    }

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
        let forwarder_a_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_a_ibc_forwarder_address,
        );
        let forwarder_b_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_b_ibc_forwarder_address,
        );
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        DEFAULT_KEY,
        &party_a_deposit_address,
        &[Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(uatom_contribution_amount),
        }],
        &Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();
    send(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        let holder_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    loop {
        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        if liquid_pooler_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && liquid_pooler_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            break;
        }
        {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until Liquid Pooler Proxy is created...");
    let proxy_address;
    loop {
        let liquid_pooler_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler state: {:?}", liquid_pooler_state);
        if liquid_pooler_state == "proxy_created" {
            let query_response = contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &liquid_pooler_address,
                &serde_json::to_string(&valence_osmo_liquid_pooler::msg::QueryMsg::ProxyAddress {})
                    .unwrap(),
            );

            proxy_address = query_response["data"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            info!(target: target,"Proxy address: {}", proxy_address);
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until Proxy is funded...");
    loop {
        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        if proxy_balance
            .iter()
            .any(|c| c.denom == atom_on_osmosis || c.denom == osmo_denom)
        {
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until liquidity is provided and proxy receives gamm tokens");
    loop {
        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler balance: {:?}", liquid_pooler_balance);

        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        let liquid_pooler_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler state: {:?}", liquid_pooler_state);

        let osmosis_outpost_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmosis_outpost_contract.address,
        );
        info!(target: target,"Osmosis Outpost balance: {:?}", osmosis_outpost_balance);
        if proxy_balance.len() == 1
            && proxy_balance.first().unwrap().denom == format!("gamm/pool/{pool_id}")
        {
            break;
        } else {
            tick(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                DEFAULT_KEY,
                &clock_address,
            );
        }
    }

    info!(target: target,"Tick until holder expires...");
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
        let holder_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        info!(target: target,"Holder state: {:?}", holder_state);
        if holder_state == "expired" {
            break;
        }
    }

    info!(target: target, "Osmosis party claims");
    thread::sleep(Duration::from_secs(10));
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();

    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );

        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler balance: {:?}", liquid_pooler_balance);

        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        let holder_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        info!(target: target,"Holder balance: {:?}", holder_balance);

        let osmo_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            ACC2_ADDRESS_OSMO,
        );
        info!(target: target,"Osmo receiver balances: {:?}", osmo_receiver_balances);
        if osmo_receiver_balances
            .iter()
            .any(|c| c.denom == atom_on_osmosis)
        {
            info!(target: target, "Osmosis party received the funds");
            break;
        }
    }

    info!(target: target, "Tick until we are able to withdraw");

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(10));

    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );

        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );
        info!(target: target,"Liquid Pooler balance: {:?}", liquid_pooler_balance);

        let proxy_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &proxy_address,
        );
        info!(target: target,"Proxy balance: {:?}", proxy_balance);

        let holder_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        info!(target: target,"Holder balance: {:?}", holder_balance);

        let hub_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            ACC1_ADDRESS_GAIA,
        );
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == osmo_on_gaia)
        {
            info!(target: target, "Both parties received the funds");
            break;
        }
    }

    // Send the balances back so we have a fresh start for the next test
    send_non_native_balances(
        test_ctx,
        GAIA_CHAIN_NAME,
        ACC_1_KEY,
        ACC1_ADDRESS_GAIA,
        NEUTRON_CHAIN_ADMIN_ADDR,
        &atom_denom,
    );

    send_non_native_balances(
        test_ctx,
        OSMOSIS_CHAIN_NAME,
        ACC_2_KEY,
        ACC2_ADDRESS_OSMO,
        NEUTRON_CHAIN_ADMIN_ADDR,
        &osmo_denom,
    );

    info!("Finished two party POL Osmosis tests!");
    Ok(())
}
