use crate::helpers::constants::{
    ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH,
};
use cosmwasm_std::{Coin, Decimal, Uint128, Uint64};
use covenant_utils::{op_mode::ContractOperationModeConfig, split::SplitConfig};
use localic_std::{errors::LocalError, modules::bank, modules::cosmwasm::CosmWasm};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, GAIA_CHAIN_NAME, NEUTRON_CHAIN_ID,
    NEUTRON_CHAIN_NAME,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
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

/// Tests that the remote chain splitter correctly handles
/// - Normal case
/// - Timeouts (does not advance to ica_created)
/// - Timeout recovery (advances after on 2nd tick after a packet timeout)
pub fn test_remote_chain_splitter(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    info!("Starting remote chain splitter tests...");

    upload_contracts(test_ctx)?;

    // See above: 3 tests, each with a different splitter
    let splitters = (
        make_remote_chain_splitter(test_ctx)?,
        make_remote_chain_splitter(test_ctx)?,
        make_remote_chain_splitter(test_ctx)?,
    );
    fund_remote_chain_splitter(test_ctx, &splitters.0)?;
    fund_remote_chain_splitter(test_ctx, &splitters.1)?;
    fund_remote_chain_splitter(test_ctx, &splitters.2)?;

    // Separate tests: ensure that the splitter can handle the happy case
    // and a timeout (independently)
    test_remote_chain_splitter_ok(test_ctx, &splitters.0)?;
    test_remote_chain_splitter_timeout(test_ctx, &splitters.1)?;

    // Combined test: runs test_timeout and test_ok in sequence with the same splitter
    // to ensure that the splitter can recover from a timeout
    test_remote_chain_splitter_timeout_recover(test_ctx, &splitters.2)?;

    info!("Finished remote chain splitter tests!");

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

fn make_remote_chain_splitter(test_ctx: &mut TestContext) -> Result<String, LocalError> {
    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN_NAME).get();
    let uatom_contribution_amount: u128 = 5_000_000_000;

    let split_config: BTreeMap<String, SplitConfig> = BTreeMap::from([(
        atom_denom.clone(),
        SplitConfig {
            receivers: BTreeMap::from([
                (ACC1_ADDRESS_NEUTRON.to_owned(), Decimal::percent(50)),
                (ACC2_ADDRESS_NEUTRON.to_owned(), Decimal::percent(50)),
            ]),
        },
    )]);

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
                splits: split_config,
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

    Ok(remote_chain_splitter.contract_addr.unwrap())
}

fn fund_remote_chain_splitter(
    test_ctx: &mut TestContext,
    remote_chain_splitter_addr: &str,
) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    bank::send(
        &neutron.rb,
        "acc0",
        remote_chain_splitter_addr,
        &[Coin::new(1000000, "untrn")],
        &Coin::new(4206942, "untrn"),
    )
    .unwrap();

    Ok(())
}

/// Tests that the splitter does not advance when there is no relayer available.
/// Leaves the relayer intact after its execution.
fn test_remote_chain_splitter_timeout(
    test_ctx: &mut TestContext,
    remote_chain_splitter: &str,
) -> Result<(), LocalError> {
    // Stop the relayer
    stop_relayer(test_ctx);

    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    let remote_chain_splitter_contract = CosmWasm::new_from_existing(
        &neutron.rb,
        None,
        None,
        Some(remote_chain_splitter.to_owned()),
    );

    // Kill the relayer and advance the splitter.
    // This should trigger SudoMsg::Timeout, which returns the state to instantiated

    // Continuously tick the splitter until the state advances to timed out (after we expect the timeout)
    with_poll_timeout! {
        {
            remote_chain_splitter_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_remote_chain_splitter::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();
        }
    }

    assert!(
        remote_chain_splitter_contract
            .query(
                &serde_json::to_string(
                    &valence_remote_chain_splitter::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data")
            == Some(&Value::String("instantiated".to_owned()))
    );

    start_relayer(test_ctx);

    Ok(())
}

/// Tests that the splitter advances after 2nd tick after a packet timeout.
fn test_remote_chain_splitter_timeout_recover(
    test_ctx: &mut TestContext,
    remote_chain_splitter: &str,
) -> Result<(), LocalError> {
    // Stop the relayer and ensure the splitter does not advance beyond instantiated
    stop_relayer(test_ctx);

    test_remote_chain_splitter_timeout(test_ctx, remote_chain_splitter)?;

    // Ensure that the splitter recovers after the relayer is started again
    start_relayer(test_ctx);

    test_remote_chain_splitter_ok(test_ctx, remote_chain_splitter)?;

    Ok(())
}

fn test_remote_chain_splitter_ok(
    test_ctx: &mut TestContext,
    remote_chain_splitter: &str,
) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    // Advance the splitter.
    // This should trigger SudoMsg::OpenAck, which will set the ContractState to IcaCreated

    // The state should be IcaCreated
    let remote_chain_splitter_contract = CosmWasm::new_from_existing(
        &neutron.rb,
        None,
        None,
        Some(remote_chain_splitter.to_owned()),
    );

    with_poll_timeout! {
        {
             remote_chain_splitter_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_remote_chain_splitter::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();

            if remote_chain_splitter_contract
                .query(
                    &serde_json::to_string(&valence_remote_chain_splitter::msg::QueryMsg::ContractState {})
                        .unwrap(),
                )
                .get("data") == Some(&Value::String("ica_created".to_owned()))
            {
                break;
            }
        }
    }

    assert_eq!(
        remote_chain_splitter_contract
            .query(
                &serde_json::to_string(
                    &valence_remote_chain_splitter::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data"),
        Some(&Value::String("ica_created".to_owned()))
    );

    Ok(())
}
