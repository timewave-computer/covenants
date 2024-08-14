use crate::helpers::constants::{LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH};
use cosmwasm_std::{Coin, Uint64};
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

/// Tests that the liquid staker correctly handles
/// - Normal case
/// - Timeouts (does not advance to ica_created)
/// - Timeout recovery (advances after on 2nd tick after a packet timeout)
pub fn test_liquid_staker(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    info!("Starting liquid staker tests...");

    upload_contracts(test_ctx)?;

    // See above: 3 tests, each with a different staker
    let stakers = (
        make_liquid_staker(test_ctx)?,
        make_liquid_staker(test_ctx)?,
        make_liquid_staker(test_ctx)?,
    );
    fund_liquid_staker(test_ctx, &stakers.0)?;
    fund_liquid_staker(test_ctx, &stakers.1)?;
    fund_liquid_staker(test_ctx, &stakers.2)?;

    // Separate tests: ensure that the staker can handle the happy case
    // and a timeout (independently)
    test_staker_ok(test_ctx, &stakers.0)?;
    test_staker_timeout(test_ctx, &stakers.1)?;

    // Combined test: runs test_timeout and test_ok in sequence with the same staker
    // to ensure that the staker can recover from a timeout
    test_staker_timeout_recover(test_ctx, &stakers.2)?;

    info!("Finished liquid staker tests!");

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
        .send_with_local_cache(VALENCE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    Ok(())
}

fn make_liquid_staker(test_ctx: &mut TestContext) -> Result<String, LocalError> {
    let mut liquid_pooler = test_ctx
        .get_contract("valence_outpost_osmo_liquid_pooler")
        .unwrap();

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

    let mut liquid_staker = test_ctx
        .get_contract("valence_stride_liquid_staker")
        .unwrap();

    let stride_neutron = test_ctx
        .get_transfer_channels()
        .src(NEUTRON_CHAIN_NAME)
        .dest(STRIDE_CHAIN_NAME)
        .get();
    let neutron_stride_conn_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(STRIDE_CHAIN_NAME)
        .get();

    liquid_staker
        .instantiate(
            DEFAULT_KEY,
            serde_json::to_string(&valence_stride_liquid_staker::msg::InstantiateMsg {
                op_mode_cfg: ContractOperationModeConfig::Permissionless,
                stride_neutron_ibc_transfer_channel_id: stride_neutron,
                neutron_stride_ibc_connection_id: neutron_stride_conn_id,
                next_contract: liquid_pooler.contract_addr.unwrap(),
                ls_denom: String::from("stuatom"),
                ica_timeout: Uint64::new(DEFAULT_TIMEOUT),
                ibc_transfer_timeout: Uint64::new(DEFAULT_TIMEOUT),
            })
            .unwrap()
            .as_str(),
            "valence_liquid_staker",
            None,
            "",
        )
        .unwrap();

    Ok(liquid_staker.contract_addr.unwrap())
}

fn fund_liquid_staker(test_ctx: &mut TestContext, staker_addr: &str) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    bank::send(
        &neutron.rb,
        "acc0",
        staker_addr,
        &[Coin::new(1000000, "untrn")],
        &Coin::new(4206942, "untrn"),
    )
    .unwrap();

    Ok(())
}

/// Tests that the staker does not advance when there is no relayer available.
/// Leaves the relayer intact after its execution.
fn test_staker_timeout(test_ctx: &mut TestContext, staker_addr: &str) -> Result<(), LocalError> {
    // Stop the relayer
    stop_relayer(test_ctx);

    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    let staker_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(staker_addr.to_owned()));

    // Kill the relayer and advance the staker.
    // This should trigger SudoMsg::Timeout, which returns the state to instantiated

    // Continuously tick the staker until the state advances to timed out (after we expect the timeout)
    with_poll_timeout! {
        {
            staker_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_stride_liquid_staker::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();
        }
    }

    assert!(
        staker_contract
            .query(
                &serde_json::to_string(
                    &valence_stride_liquid_staker::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data")
            == Some(&Value::String("instantiated".to_owned()))
    );

    start_relayer(test_ctx);

    Ok(())
}

/// Tests that the staker advances after 2nd tick after a packet timeout.
fn test_staker_timeout_recover(
    test_ctx: &mut TestContext,
    staker_addr: &str,
) -> Result<(), LocalError> {
    // Stop the relayer and ensure the staker does not advance beyond instantiated
    stop_relayer(test_ctx);

    test_staker_timeout(test_ctx, staker_addr)?;

    // Ensure that the staker recovers after the relayer is started again
    start_relayer(test_ctx);

    test_staker_ok(test_ctx, staker_addr)?;

    Ok(())
}

fn test_staker_ok(test_ctx: &mut TestContext, staker_addr: &str) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    // Advance the staker.
    // This should trigger SudoMsg::OpenAck, which will set the ContractState to IcaCreated

    // The state should be IcaCreated
    let staker_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(staker_addr.to_owned()));

    with_poll_timeout! {
        {
             staker_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_stride_liquid_staker::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();

            if staker_contract
                .query(
                    &serde_json::to_string(&valence_stride_liquid_staker::msg::QueryMsg::ContractState {})
                        .unwrap(),
                )
                .get("data") == Some(&Value::String("ica_created".to_owned()))
            {
                break;
            }
        }
    }

    assert_eq!(
        staker_contract
            .query(
                &serde_json::to_string(
                    &valence_stride_liquid_staker::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data"),
        Some(&Value::String("ica_created".to_owned()))
    );

    Ok(())
}
