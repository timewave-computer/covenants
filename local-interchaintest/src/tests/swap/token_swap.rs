use std::collections::BTreeMap;

use cosmwasm_std::{Coin, Decimal, Uint128, Uint64};
use covenant_utils::{split::SplitConfig, InterchainCovenantParty, NativeCovenantParty};
use cw_utils::Expiration;
use localic_std::{
    errors::LocalError,
    modules::{
        bank::{get_balance, send},
        cosmwasm::contract_instantiate,
    },
    node::Chain,
};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, GAIA_CHAIN_ADMIN_ADDR, GAIA_CHAIN_NAME,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_covenant_swap::msg::{CovenantPartyConfig, SwapCovenantContractCodeIds, Timeouts};

use crate::{
    helpers::{
        common::{query_contract_state, tick},
        constants::{
            ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, ACC_1_KEY, ACC_2_KEY,
            LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH,
        },
        covenant::Covenant,
    },
    send_non_native_balances,
};

pub fn test_token_swap(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN_NAME).get();
    let neutron_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN_NAME).get();
    let atom_on_neutron = test_ctx.get_ibc_denom(&atom_denom, GAIA_CHAIN_NAME, NEUTRON_CHAIN_NAME);
    let neutron_on_gaia =
        test_ctx.get_ibc_denom(&neutron_denom, NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME);

    let valence_ibc_forwarder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_ibc_forwarder")
        .unwrap();

    let valence_swap_holder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_swap_holder")
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

    let valence_native_splitter_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_native_splitter")
        .unwrap();

    let valence_covenant_swap_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_covenant_swap")
        .unwrap();

    let uatom_contribution_amount: u128 = 5_000_000_000;
    let untrn_contribution_amount: u128 = 100_000_000_000;

    // Instantiate covenant
    info!("Starting swap covenant test...");
    let current_block_height = Chain::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    )
    .get_height();

    let covenant_instantiate_msg = valence_covenant_swap::msg::InstantiateMsg {
        label: "swap-covenant".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(10000),          // seconds
            ibc_transfer_timeout: Uint64::new(10000), // seconds
        },
        contract_codes: SwapCovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            splitter_code: valence_native_splitter_code_id,
            holder_code: valence_swap_holder_code_id,
            clock_code: valence_clock_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 350),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(10000),
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
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Native(NativeCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: neutron_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            contribution: Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(untrn_contribution_amount),
            },
        }),
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(100)),
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(0)),
                    ]),
                },
            ),
            (
                neutron_denom.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(0)),
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(100)),
                    ]),
                },
            ),
        ]),
        fallback_split: None,
        fallback_address: None,
    };

    let covenant_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        valence_covenant_swap_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "swap-covenant",
        None,
        "",
    )?;
    info!("Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::Swap {
        rb: test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let splitter_address = covenant.query_splitter_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!("Fund covenant addresses with NTRN...");
    let mut addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        splitter_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
    ];
    if !party_a_ibc_forwarder_address.is_empty() {
        addresses.push(party_a_ibc_forwarder_address.clone());
    }
    if !party_b_ibc_forwarder_address.is_empty() {
        addresses.push(party_b_ibc_forwarder_address.clone());
    }
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

    info!("Tick until forwarders create ICA...");
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
        info!("Forwarder A state: {:?}", forwarder_a_state);
        if forwarder_a_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }

    info!("Party A deposit address: {}", party_a_deposit_address);
    info!("Party B deposit address: {}", party_b_deposit_address);

    info!("Fund the forwarders with sufficient funds...");
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
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(untrn_contribution_amount),
        }],
        &Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until forwarders forward the funds to the holder...");
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
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Holder received ATOM & NTRN");
            break;
        } else if holder_state == "complete" {
            info!("Holder is complete");
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

    info!("Tick until holder sends the funds to splitter");
    loop {
        let splitter_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );
        info!("Splitter balance: {:?}", splitter_balance);
        if splitter_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && splitter_balance.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Splitter received contributions");
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

    info!("Tick until splitter sends funds to routers");
    loop {
        let router_a_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_a_router_address,
        );
        info!("Router A balances: {:?}", router_a_balances);
        let router_b_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &party_b_router_address,
        );
        info!("Router B balances: {:?}", router_b_balances);
        if router_a_balances.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) && router_b_balances.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) {
            info!("Routers received contributions");
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

    info!("Tick until routers route the funds to final receivers");
    loop {
        let hub_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            ACC1_ADDRESS_GAIA,
        );
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            ACC2_ADDRESS_NEUTRON,
        );
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == neutron_on_gaia.clone())
            && neutron_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_neutron.clone())
        {
            info!("Final receivers received their funds!");
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
        NEUTRON_CHAIN_NAME,
        ACC_2_KEY,
        ACC2_ADDRESS_NEUTRON,
        GAIA_CHAIN_ADMIN_ADDR,
        &neutron_denom,
    );

    info!("Finished swap covenant test!");

    Ok(())
}
