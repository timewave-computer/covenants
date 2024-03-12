use std::collections::BTreeMap;

use cosmwasm_std::{coin, to_json_binary, Addr, Event, Uint64};

use crate::setup::{base_suite::BaseSuiteMut, DENOM_ATOM, DENOM_ATOM_ON_NTRN, NTRN_HUB_CHANNEL};

use super::suite::TwoPartyCovenantBuilder;

#[test]
fn test_instantiate_both_native_parties_astroport() {
    let _suite = TwoPartyCovenantBuilder::default().build();
}

#[test]
fn test_instantiate_party_a_interchain() {
    let builder = TwoPartyCovenantBuilder::default();
    let party_address = builder
        .instantiate_msg
        .msg
        .party_a_config
        .get_final_receiver_address();
    builder
        .with_party_a_config(
            covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
                covenant_utils::InterchainCovenantParty {
                    party_receiver_addr: party_address.to_string(),
                    party_chain_connection_id: "connection-0".to_string(),
                    ibc_transfer_timeout: Uint64::new(100),
                    party_to_host_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                    host_to_party_chain_channel_id: NTRN_HUB_CHANNEL.1.to_string(),
                    remote_chain_denom: DENOM_ATOM.to_string(),
                    addr: party_address.to_string(),
                    native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                    contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                    denom_to_pfm_map: BTreeMap::new(),
                },
            ),
        )
        .build();
}

#[test]
fn test_instantiate_party_b_interchain() {
    let builder = TwoPartyCovenantBuilder::default();
    let party_address = builder
        .instantiate_msg
        .msg
        .party_b_config
        .get_final_receiver_address();
    builder
        .with_party_b_config(
            covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
                covenant_utils::InterchainCovenantParty {
                    party_receiver_addr: party_address.to_string(),
                    party_chain_connection_id: "connection-0".to_string(),
                    ibc_transfer_timeout: Uint64::new(100),
                    party_to_host_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                    host_to_party_chain_channel_id: NTRN_HUB_CHANNEL.1.to_string(),
                    remote_chain_denom: DENOM_ATOM.to_string(),
                    addr: party_address.to_string(),
                    native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                    contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                    denom_to_pfm_map: BTreeMap::new(),
                },
            ),
        )
        .build();
}

#[test]
fn test_instantiate_with_fallback_split() {
    let builder = TwoPartyCovenantBuilder::default();
    let fallback_split = builder
        .instantiate_msg
        .msg
        .splits
        .get(&DENOM_ATOM_ON_NTRN.to_string())
        .unwrap()
        .clone();
    builder.with_fallback_split(Some(fallback_split)).build();
}

#[test]
fn test_migrate_update_config_party_a_interchain() {
    let builder = TwoPartyCovenantBuilder::default();
    let party_address = builder
        .instantiate_msg
        .msg
        .party_a_config
        .get_final_receiver_address();
    let mut suite = builder
        .with_party_a_config(
            covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
                covenant_utils::InterchainCovenantParty {
                    party_receiver_addr: party_address.to_string(),
                    party_chain_connection_id: "connection-0".to_string(),
                    ibc_transfer_timeout: Uint64::new(100),
                    party_to_host_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                    host_to_party_chain_channel_id: NTRN_HUB_CHANNEL.1.to_string(),
                    remote_chain_denom: DENOM_ATOM.to_string(),
                    addr: party_address.to_string(),
                    native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                    contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                    denom_to_pfm_map: BTreeMap::new(),
                },
            ),
        )
        .build();
    let random_address = suite.faucet.clone();

    let clock_migrate_msg = covenant_clock::msg::MigrateMsg::UpdateTickMaxGas {
        new_value: Uint64::new(500_000),
    };
    let holder_migrate_msg = covenant_two_party_pol_holder::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        next_contract: None,
        emergency_committee: None,
        lockup_config: None,
        deposit_deadline: None,
        ragequit_config: None.into(),
        covenant_config: None.into(),
        denom_splits: None,
        fallback_split: None,
    };
    let astro_liquid_pooler_migrate_msg =
        covenant_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(random_address.to_string()),
            holder_address: None,
            lp_config: None,
        };

    let liquid_pooler_migrate_msg = covenant_two_party_pol::msg::LiquidPoolerMigrateMsg::Astroport(
        astro_liquid_pooler_migrate_msg.clone(),
    );
    let party_a_interchain_router_migrate_msg =
        covenant_interchain_router::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(random_address.to_string()),
            destination_config: None,
            target_denoms: None,
        };
    let party_a_router_migrate_msg = covenant_two_party_pol::msg::RouterMigrateMsg::Interchain(
        party_a_interchain_router_migrate_msg.clone(),
    );
    let party_b_native_router_migrate_msg = covenant_native_router::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        receiver_address: None,
        target_denoms: None,
    };
    let party_b_router_migrate_msg = covenant_two_party_pol::msg::RouterMigrateMsg::Native(
        party_b_native_router_migrate_msg.clone(),
    );
    let party_a_forwarder_migrate_msg = covenant_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        next_contract: None,
        remote_chain_info: None.into(),
        transfer_amount: None,
    };

    let resp = suite.migrate_update(
        21,
        covenant_two_party_pol::msg::MigrateMsg::UpdateCovenant {
            clock: Some(clock_migrate_msg.clone()),
            holder: Some(holder_migrate_msg.clone()),
            liquid_pooler: Some(liquid_pooler_migrate_msg.clone()),
            party_a_router: Some(party_a_router_migrate_msg.clone()),
            party_b_router: Some(party_b_router_migrate_msg.clone()),
            party_a_forwarder: Some(party_a_forwarder_migrate_msg.clone()),
            party_b_forwarder: None,
        },
    );

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute(
                "clock_migrate",
                to_json_binary(&clock_migrate_msg).unwrap().to_string(),
            )
            .add_attribute(
                "party_a_router_migrate",
                to_json_binary(&party_a_interchain_router_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "party_b_router_migrate",
                to_json_binary(&party_b_native_router_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "party_a_forwarder_migrate",
                to_json_binary(&party_a_forwarder_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "holder_migrate",
                to_json_binary(&holder_migrate_msg).unwrap().to_string(),
            )
            .add_attribute(
                "liquid_pooler_migrate",
                to_json_binary(&astro_liquid_pooler_migrate_msg)
                    .unwrap()
                    .to_string(),
            ),
    );

    let clock_address = suite.query_clock_address();
    let holder_address = suite.query_holder_address();
    let liquid_pooler_address = suite.query_liquid_pooler_address();
    let party_a_router_address = suite.query_interchain_router_address("party_a");
    let party_b_router_address = suite.query_interchain_router_address("party_b");
    let party_a_forwarder_address = suite.query_ibc_forwarder_address("party_a");

    suite.tick_contract(suite.clock_addr.clone());

    let app = suite.get_app();

    let clock_max_gas: Uint64 = app
        .wrap()
        .query_wasm_smart(clock_address, &covenant_clock::msg::QueryMsg::TickMaxGas {})
        .unwrap();
    assert_eq!(clock_max_gas, Uint64::new(500_000));

    let holder_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            holder_address,
            &covenant_two_party_pol_holder::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(holder_clock_address, random_address);

    let liquid_pooler_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            liquid_pooler_address,
            &covenant_astroport_liquid_pooler::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(liquid_pooler_clock_address, random_address);

    let party_a_router_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_a_router_address,
            &covenant_interchain_router::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_a_router_clock_address, random_address);

    let party_b_router_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_b_router_address,
            &covenant_native_router::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_b_router_clock_address, random_address);

    let party_a_forwarder_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_a_forwarder_address,
            &covenant_ibc_forwarder::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_a_forwarder_clock_address, random_address);
}

#[test]
fn test_migrate_update_config_party_b_interchain() {
    let builder = TwoPartyCovenantBuilder::default();
    let party_address = builder
        .instantiate_msg
        .msg
        .party_b_config
        .get_final_receiver_address();
    let mut suite = builder
        .with_party_b_config(
            covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
                covenant_utils::InterchainCovenantParty {
                    party_receiver_addr: party_address.to_string(),
                    party_chain_connection_id: "connection-0".to_string(),
                    ibc_transfer_timeout: Uint64::new(100),
                    party_to_host_chain_channel_id: NTRN_HUB_CHANNEL.0.to_string(),
                    host_to_party_chain_channel_id: NTRN_HUB_CHANNEL.1.to_string(),
                    remote_chain_denom: DENOM_ATOM.to_string(),
                    addr: party_address.to_string(),
                    native_denom: DENOM_ATOM_ON_NTRN.to_string(),
                    contribution: coin(10_000, DENOM_ATOM_ON_NTRN),
                    denom_to_pfm_map: BTreeMap::new(),
                },
            ),
        )
        .build();
    let random_address = suite.faucet.clone();

    let clock_migrate_msg = covenant_clock::msg::MigrateMsg::UpdateTickMaxGas {
        new_value: Uint64::new(500_000),
    };
    let holder_migrate_msg = covenant_two_party_pol_holder::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        next_contract: None,
        emergency_committee: None,
        lockup_config: None,
        deposit_deadline: None,
        ragequit_config: None.into(),
        covenant_config: None.into(),
        denom_splits: None,
        fallback_split: None,
    };
    let astro_liquid_pooler_migrate_msg =
        covenant_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(random_address.to_string()),
            holder_address: None,
            lp_config: None,
        };

    let liquid_pooler_migrate_msg = covenant_two_party_pol::msg::LiquidPoolerMigrateMsg::Astroport(
        astro_liquid_pooler_migrate_msg.clone(),
    );
    let party_b_interchain_router_migrate_msg =
        covenant_interchain_router::msg::MigrateMsg::UpdateConfig {
            clock_addr: Some(random_address.to_string()),
            destination_config: None,
            target_denoms: None,
        };
    let party_b_router_migrate_msg = covenant_two_party_pol::msg::RouterMigrateMsg::Interchain(
        party_b_interchain_router_migrate_msg.clone(),
    );
    let party_a_native_router_migrate_msg = covenant_native_router::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        receiver_address: None,
        target_denoms: None,
    };
    let party_a_router_migrate_msg = covenant_two_party_pol::msg::RouterMigrateMsg::Native(
        party_a_native_router_migrate_msg.clone(),
    );
    let party_b_forwarder_migrate_msg = covenant_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
        clock_addr: Some(random_address.to_string()),
        next_contract: None,
        remote_chain_info: None.into(),
        transfer_amount: None,
    };

    let resp = suite.migrate_update(
        21,
        covenant_two_party_pol::msg::MigrateMsg::UpdateCovenant {
            clock: Some(clock_migrate_msg.clone()),
            holder: Some(holder_migrate_msg.clone()),
            liquid_pooler: Some(liquid_pooler_migrate_msg.clone()),
            party_a_router: Some(party_a_router_migrate_msg.clone()),
            party_b_router: Some(party_b_router_migrate_msg.clone()),
            party_b_forwarder: Some(party_b_forwarder_migrate_msg.clone()),
            party_a_forwarder: None,
        },
    );

    resp.assert_event(
        &Event::new("wasm")
            .add_attribute(
                "clock_migrate",
                to_json_binary(&clock_migrate_msg).unwrap().to_string(),
            )
            .add_attribute(
                "party_b_router_migrate",
                to_json_binary(&party_b_interchain_router_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "party_a_router_migrate",
                to_json_binary(&party_a_native_router_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "party_b_forwarder_migrate",
                to_json_binary(&party_b_forwarder_migrate_msg)
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "holder_migrate",
                to_json_binary(&holder_migrate_msg).unwrap().to_string(),
            )
            .add_attribute(
                "liquid_pooler_migrate",
                to_json_binary(&astro_liquid_pooler_migrate_msg)
                    .unwrap()
                    .to_string(),
            ),
    );

    let clock_address = suite.query_clock_address();
    let holder_address = suite.query_holder_address();
    let liquid_pooler_address = suite.query_liquid_pooler_address();
    let party_a_router_address = suite.query_interchain_router_address("party_a");
    let party_b_router_address = suite.query_interchain_router_address("party_b");
    let party_b_forwarder_address = suite.query_ibc_forwarder_address("party_b");

    suite.tick_contract(suite.clock_addr.clone());

    let app = suite.get_app();

    let clock_max_gas: Uint64 = app
        .wrap()
        .query_wasm_smart(clock_address, &covenant_clock::msg::QueryMsg::TickMaxGas {})
        .unwrap();
    assert_eq!(clock_max_gas, Uint64::new(500_000));

    let holder_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            holder_address,
            &covenant_two_party_pol_holder::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(holder_clock_address, random_address);

    let liquid_pooler_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            liquid_pooler_address,
            &covenant_astroport_liquid_pooler::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(liquid_pooler_clock_address, random_address);

    let party_b_router_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_b_router_address,
            &covenant_interchain_router::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_b_router_clock_address, random_address);

    let party_a_router_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_a_router_address,
            &covenant_native_router::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_a_router_clock_address, random_address);

    let party_b_forwarder_clock_address: Addr = app
        .wrap()
        .query_wasm_smart(
            party_b_forwarder_address,
            &covenant_ibc_forwarder::msg::QueryMsg::ClockAddress {},
        )
        .unwrap();
    assert_eq!(party_b_forwarder_clock_address, random_address);
}
