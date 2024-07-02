use std::collections::BTreeMap;

use cosmwasm_std::{coin, to_json_binary, Event, Uint64};
use covenant_utils::op_mode::{ContractOperationMode, ContractOperationModeConfig};

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
            valence_covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
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
                    fallback_address: None,
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
            valence_covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
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
                    fallback_address: None,
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
        .get(DENOM_ATOM_ON_NTRN)
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
            valence_covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
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
                    fallback_address: None,
                },
            ),
        )
        .build();
    let random_address = suite.faucet.clone();

    let holder_migrate_msg = valence_two_party_pol_holder::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
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
        valence_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
            op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
                random_address.to_string(),
            ])),
            holder_address: None,
            lp_config: None,
        };

    let liquid_pooler_migrate_msg =
        valence_covenant_two_party_pol::msg::LiquidPoolerMigrateMsg::Astroport(
            astro_liquid_pooler_migrate_msg.clone(),
        );
    let party_a_interchain_router_migrate_msg =
        valence_interchain_router::msg::MigrateMsg::UpdateConfig {
            op_mode: ContractOperationModeConfig::Permissioned(vec![random_address.to_string()])
                .into(),
            destination_config: None,
            target_denoms: None,
        };
    let party_a_router_migrate_msg =
        valence_covenant_two_party_pol::msg::RouterMigrateMsg::Interchain(
            party_a_interchain_router_migrate_msg.clone(),
        );
    let party_b_native_router_migrate_msg = valence_native_router::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
        receiver_address: None,
        target_denoms: None,
    };
    let party_b_router_migrate_msg = valence_covenant_two_party_pol::msg::RouterMigrateMsg::Native(
        party_b_native_router_migrate_msg.clone(),
    );
    let party_a_forwarder_migrate_msg = valence_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
        next_contract: None,
        remote_chain_info: None.into(),
        transfer_amount: None,
        fallback_address: None,
    };
    let mut contract_codes = suite.query_contract_codes();
    contract_codes.clock = 1;
    let resp = suite.migrate_update(
        22,
        valence_covenant_two_party_pol::msg::MigrateMsg::UpdateCovenant {
            codes: Some(contract_codes.clone()),
            clock: None,
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
                "contract_codes_migrate",
                to_json_binary(&contract_codes).unwrap().to_string(),
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

    let holder_address = suite.query_holder_address();
    let liquid_pooler_address = suite.query_liquid_pooler_address();
    let party_a_router_address = suite.query_interchain_router_address("party_a");
    let party_b_router_address = suite.query_interchain_router_address("party_b");
    let party_a_forwarder_address = suite.query_ibc_forwarder_address("party_a");
    let new_contract_codes = suite.query_contract_codes();

    suite.tick_contract(suite.clock_addr.clone());

    let app = suite.get_app();

    let holder_op_mode = app
        .wrap()
        .query_wasm_smart(
            holder_address,
            &valence_two_party_pol_holder::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    let holder_clock_address = match holder_op_mode {
        ContractOperationMode::Permissioned(addr) => addr.to_vec()[0].clone(),
        _ => panic!("unexpected op mode"),
    };
    assert_eq!(holder_clock_address, random_address);

    let liquid_pooler_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            liquid_pooler_address,
            &valence_astroport_liquid_pooler::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        liquid_pooler_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_a_router_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_a_router_address,
            &valence_interchain_router::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_a_router_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_b_router_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_b_router_address,
            &valence_native_router::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_b_router_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_a_forwarder_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_a_forwarder_address,
            &valence_ibc_forwarder::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_a_forwarder_op_mode,
        ContractOperationMode::Permissioned(vec![random_address].into())
    );

    assert_eq!(new_contract_codes, contract_codes);
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
            valence_covenant_two_party_pol::msg::CovenantPartyConfig::Interchain(
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
                    fallback_address: None,
                },
            ),
        )
        .build();
    let random_address = suite.faucet.clone();

    let clock_migrate_msg = valence_clock::msg::MigrateMsg::UpdateTickMaxGas {
        new_value: Uint64::new(500_000),
    };
    let holder_migrate_msg = valence_two_party_pol_holder::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
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
        valence_astroport_liquid_pooler::msg::MigrateMsg::UpdateConfig {
            op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
                random_address.to_string(),
            ])),
            holder_address: None,
            lp_config: None,
        };

    let liquid_pooler_migrate_msg =
        valence_covenant_two_party_pol::msg::LiquidPoolerMigrateMsg::Astroport(
            astro_liquid_pooler_migrate_msg.clone(),
        );
    let party_b_interchain_router_migrate_msg =
        valence_interchain_router::msg::MigrateMsg::UpdateConfig {
            op_mode: ContractOperationModeConfig::Permissioned(vec![random_address.to_string()])
                .into(),
            destination_config: None,
            target_denoms: None,
        };
    let party_b_router_migrate_msg =
        valence_covenant_two_party_pol::msg::RouterMigrateMsg::Interchain(
            party_b_interchain_router_migrate_msg.clone(),
        );
    let party_a_native_router_migrate_msg = valence_native_router::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
        receiver_address: None,
        target_denoms: None,
    };
    let party_a_router_migrate_msg = valence_covenant_two_party_pol::msg::RouterMigrateMsg::Native(
        party_a_native_router_migrate_msg.clone(),
    );
    let party_b_forwarder_migrate_msg = valence_ibc_forwarder::msg::MigrateMsg::UpdateConfig {
        op_mode: Some(ContractOperationModeConfig::Permissioned(vec![
            random_address.to_string(),
        ])),
        next_contract: None,
        remote_chain_info: None.into(),
        transfer_amount: None,
        fallback_address: None,
    };
    let mut contract_codes = suite.query_contract_codes();
    contract_codes.party_a_forwarder = 1;

    let resp = suite.migrate_update(
        22,
        valence_covenant_two_party_pol::msg::MigrateMsg::UpdateCovenant {
            codes: Some(contract_codes.clone()),
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
                "contract_codes_migrate",
                to_json_binary(&contract_codes).unwrap().to_string(),
            )
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
    let new_contract_codes = suite.query_contract_codes();
    suite.tick_contract(suite.clock_addr.clone());

    let app = suite.get_app();

    let clock_max_gas: Uint64 = app
        .wrap()
        .query_wasm_smart(clock_address, &valence_clock::msg::QueryMsg::TickMaxGas {})
        .unwrap();
    assert_eq!(clock_max_gas, Uint64::new(500_000));

    let holder_op_mode = app
        .wrap()
        .query_wasm_smart(
            holder_address,
            &valence_two_party_pol_holder::msg::QueryMsg::OperationMode {},
        )
        .unwrap();

    let holder_clock_address = match holder_op_mode {
        ContractOperationMode::Permissioned(addr) => addr.to_vec()[0].clone(),
        _ => panic!("unexpected op mode"),
    };

    assert_eq!(holder_clock_address, random_address);

    let liquid_pooler_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            liquid_pooler_address,
            &valence_astroport_liquid_pooler::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        liquid_pooler_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_b_router_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_b_router_address,
            &valence_interchain_router::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_b_router_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_a_router_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_a_router_address.clone(),
            &valence_native_router::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_a_router_op_mode,
        ContractOperationMode::Permissioned(vec![random_address.clone()].into())
    );

    let party_b_forwarder_op_mode: ContractOperationMode = app
        .wrap()
        .query_wasm_smart(
            party_b_forwarder_address,
            &valence_ibc_forwarder::msg::QueryMsg::OperationMode {},
        )
        .unwrap();
    assert_eq!(
        party_b_forwarder_op_mode,
        ContractOperationMode::Permissioned(vec![random_address].into())
    );
    assert_eq!(new_contract_codes, contract_codes);
}
