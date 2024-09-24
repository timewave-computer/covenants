use crate::helpers::constants::{LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH};
use cosmwasm_std::{Coin, Uint128, Uint64};
use covenant_utils::op_mode::ContractOperationModeConfig;
use localic_std::{errors::LocalError, modules::bank, modules::cosmwasm::CosmWasm};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
    STRIDE_CHAIN_NAME,
};
use serde_json::Value;
use std::time::{Duration, Instant};
use valence_covenant_single_party_pol::msg::DEFAULT_TIMEOUT;

use log::info;

/// How long we will wait until the ICA must have been created, or timed out
const ICA_TIMEOUT: Duration = Duration::from_secs(30);

macro_rules! with_poll_timeout {
    ($body:block) => {
        let start = Instant::now();

        while Instant::now().duration_since(start) < ICA_TIMEOUT {
            $body
        }
    };
}

/// Tests that the ibc forwarder correctly handles
/// - Normal case
/// - Timeouts (does not advance to ica_created)
/// - Timeout recovery (advances after on 2nd tick after a packet timeout)
pub fn test_ibc_forwarder(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    info!("Starting IBC forwarder tests...");

    upload_contracts(test_ctx)?;

    // See above: 3 tests, each with a different forwarder
    let forwarders = (
        make_forwarder(test_ctx)?,
        make_forwarder(test_ctx)?,
        make_forwarder(test_ctx)?,
    );
    fund_forwarder(test_ctx, &forwarders.0)?;
    fund_forwarder(test_ctx, &forwarders.1)?;
    fund_forwarder(test_ctx, &forwarders.2)?;

    // Separate tests: ensure that the forwarder can handle the happy case
    // and a timeout (independently)
    test_forwarder_ok(test_ctx, &forwarders.0)?;
    test_forwarder_timeout(test_ctx, &forwarders.1)?;

    // Combined test: runs test_timeout and test_ok in sequence with the same forwarder
    // to ensure that the forwarder can recover from a timeout
    test_forwarder_timeout_recover(test_ctx, &forwarders.2)?;

    info!("Finished IBC forwarder tests!");

    Ok(())
}

fn start_relayer(test_ctx: &mut TestContext) {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    reqwest::blocking::Client::default()
        .post(&neutron.rb.api)
        .json(&serde_json::json!({ "chain_id": NEUTRON_CHAIN_ID, "action": "start-relayer"}))
        .send()
        .unwrap();
}

fn stop_relayer(test_ctx: &mut TestContext) {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    reqwest::blocking::Client::default()
        .post(&neutron.rb.api)
        .json(&serde_json::json!({ "chain_id": NEUTRON_CHAIN_ID, "action": "stop-relayer"}))
        .send()
        .unwrap();
}

fn upload_contracts(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    Ok(())
}

fn make_forwarder(test_ctx: &mut TestContext) -> Result<String, LocalError> {
    let mut liquid_pooler = test_ctx
        .get_contract()
        .contract("valence_outpost_osmo_liquid_pooler")
        .get_cw();

    liquid_pooler
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg {})
                .unwrap(),
            "valence_outpost_osmo_liquid_pooler",
            None,
            "",
        )
        .unwrap();

    let mut forwarder = test_ctx
        .get_contract()
        .contract("valence_ibc_forwarder")
        .get_cw();

    let neutron_stride = test_ctx
        .get_transfer_channels()
        .src(NEUTRON_CHAIN_NAME)
        .dest(STRIDE_CHAIN_NAME)
        .get();
    let neutron_stride_conn_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(STRIDE_CHAIN_NAME)
        .get();

    forwarder
        .instantiate(
            DEFAULT_KEY,
            serde_json::to_string(&valence_ibc_forwarder::msg::InstantiateMsg {
                op_mode_cfg: ContractOperationModeConfig::Permissionless,
                remote_chain_channel_id: neutron_stride,
                remote_chain_connection_id: neutron_stride_conn_id,
                next_contract: liquid_pooler.contract_addr.unwrap(),
                denom: String::from("stuatom"),
                amount: Uint128::zero(),
                ica_timeout: Uint64::new(DEFAULT_TIMEOUT),
                ibc_transfer_timeout: Uint64::new(DEFAULT_TIMEOUT),
                fallback_address: None,
            })
            .unwrap()
            .as_str(),
            "valence_ibc_forwarder",
            None,
            "",
        )
        .unwrap();

    Ok(forwarder.contract_addr.unwrap())
}

fn fund_forwarder(test_ctx: &mut TestContext, forwarder_addr: &str) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    bank::send(
        &neutron.rb,
        "acc0",
        forwarder_addr,
        &[Coin::new(1000000, "untrn")],
        &Coin::new(4206942, "untrn"),
    )
    .unwrap();

    Ok(())
}

/// Tests that the forwarder does not advance when there is no relayer available.
/// Leaves the relayer intact after its execution.
fn test_forwarder_timeout(
    test_ctx: &mut TestContext,
    forwarder_addr: &str,
) -> Result<(), LocalError> {
    // Stop the relayer
    stop_relayer(test_ctx);

    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    let forwarder_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(forwarder_addr.to_owned()));

    // Kill the relayer and advance the forwarder.
    // This should trigger SudoMsg::Timeout, which returns the state to instantiated

    // Continuously tick the forwarder until the state advances to timed out (after we expect the timeout)
    with_poll_timeout! {
        {
            forwarder_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_ibc_forwarder::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();
        }
    }

    assert!(
        forwarder_contract
            .query(
                &serde_json::to_string(&valence_ibc_forwarder::msg::QueryMsg::ContractState {})
                    .unwrap()
            )
            .get("data")
            == Some(&Value::String("instantiated".to_owned()))
    );

    start_relayer(test_ctx);

    Ok(())
}

/// Tests that the forwarder advances after 2nd tick after a packet timeout.
fn test_forwarder_timeout_recover(
    test_ctx: &mut TestContext,
    forwarder_addr: &str,
) -> Result<(), LocalError> {
    // Stop the relayer and ensure the forwarder does not advance beyond instantiated
    stop_relayer(test_ctx);

    test_forwarder_timeout(test_ctx, forwarder_addr)?;

    // Ensure that the forwarder recovers after the relayer is started again
    start_relayer(test_ctx);

    test_forwarder_ok(test_ctx, forwarder_addr)?;

    Ok(())
}

fn test_forwarder_ok(test_ctx: &mut TestContext, forwarder_addr: &str) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    // Advance the forwarder.
    // This should trigger SudoMsg::OpenAck, which will set the ContractState to IcaCreated

    // The state should be IcaCreated
    let forwarder_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(forwarder_addr.to_owned()));

    with_poll_timeout! {
        {
             forwarder_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_ibc_forwarder::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();

            if forwarder_contract
                .query(
                    &serde_json::to_string(&valence_ibc_forwarder::msg::QueryMsg::ContractState {})
                        .unwrap(),
                )
                .get("data") == Some(&Value::String("ica_created".to_owned()))
            {
                break;
            }
        }
    }

    assert_eq!(
        forwarder_contract
            .query(
                &serde_json::to_string(&valence_ibc_forwarder::msg::QueryMsg::ContractState {})
                    .unwrap()
            )
            .get("data"),
        Some(&Value::String("ica_created".to_owned()))
    );

    Ok(())
}
