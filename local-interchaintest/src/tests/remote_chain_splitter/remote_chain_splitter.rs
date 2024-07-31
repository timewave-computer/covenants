use crate::helpers::constants::{ACC1_ADDRESS_GAIA, ACC2_ADDRESS_GAIA};
use cosmwasm_std::{Decimal, Uint128, Uint64};
use covenant_utils::{op_mode::ContractOperationModeConfig, split::SplitConfig};
use localic_std::{errors::LocalError, modules::cosmwasm::CosmWasm};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, GAIA_CHAIN_NAME, NEUTRON_CHAIN_NAME,
};
use std::collections::BTreeMap;
use valence_covenant_single_party_pol::msg::DEFAULT_TIMEOUT;

use log::info;

use crate::helpers::constants::{ASTROPORT_PATH, LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH};

pub fn test_remote_chain_splitter(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    info!("Starting remote chain splitter tests...");

    test_remote_chain_splitter_timeout(test_ctx)?;
    test_remote_chain_splitter_ok(test_ctx)?;

    info!("Finished remote chain splitter tests!");

    Ok(())
}

fn start_relayer(test_ctx: &mut TestContext) {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    reqwest::blocking::Client::default()
        .post(&neutron.rb.api)
        .json(&serde_json::json!({ "action": "start-relayer"}))
        .send()
        .unwrap();
}

fn stop_relayer(test_ctx: &mut TestContext) {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    reqwest::blocking::Client::default()
        .post(&neutron.rb.api)
        .json(&serde_json::json!({ "action": "stop-relayer"}))
        .send()
        .unwrap();
}

fn get_remote_chain_splitter(test_ctx: &mut TestContext) -> Result<CosmWasm, LocalError> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    uploader
        .send_with_local_cache(ASTROPORT_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN_NAME).get();
    let uatom_contribution_amount: u128 = 5_000_000_000;

    let split_config: Vec<(String, SplitConfig)> = vec![(
        atom_denom.clone(),
        SplitConfig {
            receivers: BTreeMap::from_iter(vec![
                (ACC1_ADDRESS_GAIA.to_owned(), Decimal::percent(50)),
                (ACC2_ADDRESS_GAIA.to_owned(), Decimal::percent(50)),
            ]),
        },
    )];

    let mut remote_chain_splitter = test_ctx
        .get_contract("valence_remote_chain_splitter")
        .unwrap();

    remote_chain_splitter
        .instantiate(
            DEFAULT_KEY,
            serde_json::to_string(&valence_remote_chain_splitter::msg::InstantiateMsg {
                op_mode_cfg: ContractOperationModeConfig::Permissionless,
                remote_chain_channel_id: test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(GAIA_CHAIN_NAME)
                    .get(),
                remote_chain_connection_id: test_ctx
                    .get_connections()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(GAIA_CHAIN_NAME)
                    .get(),
                denom: atom_denom.clone(),
                amount: Uint128::from(uatom_contribution_amount),
                splits: BTreeMap::from_iter(split_config),
                ica_timeout: Uint64::new(DEFAULT_TIMEOUT),
                ibc_transfer_timeout: Uint64::new(DEFAULT_TIMEOUT),
                fallback_address: None,
            })
            .unwrap()
            .as_str(),
            "valence_remote_chain_splitter",
            None,
            "",
        )
        .unwrap();

    Ok(remote_chain_splitter)
}

fn test_remote_chain_splitter_timeout(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    // Stop the relayer
    stop_relayer(test_ctx);

    let remote_chain_splitter = get_remote_chain_splitter(test_ctx)?;

    // Kill the relayer and advance the splitter.
    // This should trigger SudoMsg::Timeout, which returns the state to instantiated

    // The state should be instantiated
    remote_chain_splitter
        .execute(
            DEFAULT_KEY,
            serde_json::to_string(&valence_remote_chain_splitter::msg::ExecuteMsg::Tick {})
                .unwrap()
                .as_str(),
            "",
        )
        .unwrap();

    assert!(remote_chain_splitter
        .query(
            &serde_json::to_string(&valence_remote_chain_splitter::msg::QueryMsg::ContractState {})
                .unwrap()
        )
        .get("instantiated")
        .is_some());

    start_relayer(test_ctx);

    Ok(())
}

fn test_remote_chain_splitter_ok(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    let remote_chain_splitter = get_remote_chain_splitter(test_ctx)?;

    // Advance the splitter.
    // This should trigger SudoMsg::OpenAck, which will set the ContractState to IcaCreated

    // The state should be IcaCreated
    remote_chain_splitter
        .execute(
            DEFAULT_KEY,
            serde_json::to_string(&valence_remote_chain_splitter::msg::ExecuteMsg::Tick {})
                .unwrap()
                .as_str(),
            "",
        )
        .unwrap();

    assert!(remote_chain_splitter
        .query(
            &serde_json::to_string(&valence_remote_chain_splitter::msg::QueryMsg::ContractState {})
                .unwrap()
        )
        .get("ica_created")
        .is_some());

    Ok(())
}
