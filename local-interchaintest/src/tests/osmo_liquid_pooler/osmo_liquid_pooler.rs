use crate::helpers::constants::{
    ACC1_ADDRESS_NEUTRON, LOCAL_CODE_ID_CACHE_PATH, LOCAL_CODE_ID_CACHE_PATH_OSMO, OSMOSIS_FEES,
    POLYTONE_PATH, VALENCE_PATH,
};
use cosmwasm_std::{Coin, Decimal, Uint128, Uint64};
use covenant_utils::{op_mode::ContractOperationModeConfig, PoolPriceConfig, SingleSideLpLimits};
use cw_utils::{Duration as CwDuration, Expiration};
use localic_std::{errors::LocalError, modules::cosmwasm::CosmWasm, relayer::Relayer};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_ID,
    NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME,
};
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use valence_covenant_single_party_pol::msg::DEFAULT_TIMEOUT;
use valence_osmo_liquid_pooler::msg::{PartyChainInfo, PartyDenomInfo};

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

/// Tests that the osmo liquid pooler correctly handles
/// - Normal case
/// - Timeouts (does not advance to proxy_created)
/// - Timeout recovery (advances after on 2nd tick after a packet timeout)
pub fn test_osmo_liquid_pooler(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    info!("Starting osmo liquid pooler tests...");

    upload_contracts(test_ctx)?;

    // Tests are still deterministic. This just allows reusing the same local-ic instance between tests
    let label_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();

    let salts = (
        format!("salt1_{label_seed}"),
        format!("salt2_{label_seed}"),
        format!("salt3_{label_seed}"),
    );

    // See above: 3 tests, each with a different pooler
    let holders = (
        make_holder(test_ctx, &salts.0, &label_seed)?,
        make_holder(test_ctx, &salts.1, &label_seed)?,
        make_holder(test_ctx, &salts.2, &label_seed)?,
    );
    let poolers = (
        make_pooler(test_ctx, holders.0, &salts.0, &label_seed)?,
        make_pooler(test_ctx, holders.1, &salts.1, &label_seed)?,
        make_pooler(test_ctx, holders.2, &salts.2, &label_seed)?,
    );

    // Separate tests: ensure that the pooler can handle the happy case
    // and a timeout (independently)
    test_pooler_ok(test_ctx, &poolers.0)?;
    test_pooler_timeout(test_ctx, &poolers.1)?;

    // Combined test: runs test_timeout and test_ok in sequence with the same pooler
    // to ensure that the pooler can recover from a timeout
    test_pooler_timeout_recover(test_ctx, &poolers.2)?;

    info!("Finished osmo liquid pooler tests!");

    Ok(())
}

fn upload_contracts(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();
    uploader
        .send_with_local_cache(POLYTONE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();
    uploader
        .send_with_local_cache(
            POLYTONE_PATH,
            OSMOSIS_CHAIN_NAME,
            LOCAL_CODE_ID_CACHE_PATH_OSMO,
        )
        .unwrap();

    Ok(())
}

fn make_holder(
    test_ctx: &mut TestContext,
    salt: &str,
    label_seed: &str,
) -> Result<String, LocalError> {
    let holder_code_id = test_ctx
        .get_contract("valence_single_party_pol_holder")
        .unwrap()
        .code_id
        .unwrap();

    let pooler_address = test_ctx
        .get_built_contract_address()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_osmo_liquid_pooler")
        .salt_hex_encoded(&hex::encode(salt))
        .get();
    let holder_address = test_ctx
        .get_built_contract_address()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_single_party_pol_holder")
        .salt_hex_encoded(&hex::encode(salt))
        .get();

    test_ctx
        .build_tx_instantiate2()
        .with_label(&format!("valence_single_party_pol_holder_{label_seed}"))
        .with_code_id(holder_code_id)
        .with_salt_hex_encoded(&hex::encode(salt))
        .with_msg(
            serde_json::to_value(&valence_single_party_pol_holder::msg::InstantiateMsg {
                withdrawer: ACC1_ADDRESS_NEUTRON.to_owned(),
                withdraw_to: ACC1_ADDRESS_NEUTRON.to_owned(),
                emergency_committee_addr: None,
                pooler_address,
                lockup_period: Expiration::Never {},
            })
            .unwrap(),
        )
        .send()
        .unwrap();

    Ok(holder_address)
}

fn make_pooler(
    test_ctx: &mut TestContext,
    holder: String,
    salt: &str,
    label_seed: &str,
) -> Result<String, LocalError> {
    let pooler_code_id = test_ctx
        .get_contract("valence_osmo_liquid_pooler")
        .unwrap()
        .code_id
        .unwrap();
    let pooler_address = test_ctx
        .get_built_contract_address()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_osmo_liquid_pooler")
        .salt_hex_encoded(&hex::encode(salt))
        .get();

    fn get_contract_osmo<'a>(test_ctx: &'a mut TestContext, contract_name: &str) -> CosmWasm<'a> {
        let chain = test_ctx.get_chain(OSMOSIS_CHAIN_NAME);
        let code_id = chain.contract_codes.get(contract_name).unwrap();

        CosmWasm::new_from_existing(&chain.rb, None, Some(*code_id), None)
    }

    // Setup  osmo outpost
    let osmo_outpost_addr = {
        let mut osmo_outpost = test_ctx
            .get_contract("valence_outpost_osmo_liquid_pooler")
            .unwrap();
        osmo_outpost
            .instantiate(
                DEFAULT_KEY,
                &serde_json::to_string(&valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg {})
                    .unwrap(),
                "valence_outpost_osmo_liquid_pooler",
                None,
                "",
            )
            .unwrap();
        osmo_outpost.contract_addr.unwrap()
    };

    // Setup polytone contracts
    let polytone_proxy_code_id = get_contract_osmo(test_ctx, "polytone_proxy").code_id;
    let polytone_voice_contract_addr = {
        let mut polytone_voice = get_contract_osmo(test_ctx, "polytone_voice");
        info!("Instantiate Polytone Voice on Osmosis");

        polytone_voice
            .instantiate(
                DEFAULT_KEY,
                &serde_json::to_string(&polytone_voice::msg::InstantiateMsg {
                    proxy_code_id: Uint64::new(polytone_proxy_code_id.unwrap()),
                    block_max_gas: Uint64::new(3010000),
                })
                .unwrap(),
                "polytone-voice",
                None,
                OSMOSIS_FEES,
            )
            .unwrap();

        polytone_voice.contract_addr.unwrap()
    };

    let polytone_note_address = {
        let mut polytone_note = test_ctx.get_contract("polytone_note").unwrap();
        polytone_note
            .instantiate(
                DEFAULT_KEY,
                &serde_json::to_string(&polytone_note::msg::InstantiateMsg {
                    pair: None,
                    block_max_gas: Uint64::new(3010000),
                })
                .unwrap(),
                "polytone-note",
                None,
                "",
            )
            .unwrap();

        info!("Create polytone channel");

        let relayer = Relayer::new(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
        );

        polytone_note
            .create_wasm_connection(
                &relayer,
                "neutron-osmosis",
                &CosmWasm::new_from_existing(
                    test_ctx
                        .get_request_builder()
                        .get_request_builder(OSMOSIS_CHAIN_NAME),
                    None,
                    None,
                    Some(polytone_voice_contract_addr),
                ),
                "unordered",
                "polytone-1",
            )
            .unwrap();

        polytone_note.contract_addr.unwrap()
    };

    let neutron_to_osmosis = test_ctx
        .get_transfer_channels()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let osmosis_to_neutron = test_ctx
        .get_transfer_channels()
        .src(OSMOSIS_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    test_ctx
        .build_tx_instantiate2()
        .with_label(&format!("valence_osmo_liquid_pooler_{label_seed}"))
        .with_code_id(pooler_code_id)
        .with_salt_hex_encoded(&hex::encode(salt))
        .with_msg(
            serde_json::to_value(&valence_osmo_liquid_pooler::msg::InstantiateMsg {
                op_mode_cfg: ContractOperationModeConfig::Permissionless,
                holder_address: holder.to_owned(),
                note_address: polytone_note_address,
                pool_id: Uint64::default(),
                osmo_ibc_timeout: Uint64::new(DEFAULT_TIMEOUT),
                party_1_chain_info: PartyChainInfo {
                    party_chain_to_neutron_channel: osmosis_to_neutron.clone(),
                    neutron_to_party_chain_channel: neutron_to_osmosis.clone(),
                    outwards_pfm: None,
                    inwards_pfm: None,
                    ibc_timeout: Uint64::new(DEFAULT_TIMEOUT),
                },
                party_2_chain_info: PartyChainInfo {
                    party_chain_to_neutron_channel: osmosis_to_neutron,
                    neutron_to_party_chain_channel: neutron_to_osmosis.clone(),
                    outwards_pfm: None,
                    inwards_pfm: None,
                    ibc_timeout: Uint64::new(DEFAULT_TIMEOUT),
                },
                osmo_to_neutron_channel_id: neutron_to_osmosis,
                party_1_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin::new(0, "uosmo"),
                    local_denom: "untrn".to_owned(),
                },
                party_2_denom_info: PartyDenomInfo {
                    osmosis_coin: Coin::new(0, "uosmo"),
                    local_denom: "untrn".to_owned(),
                },
                osmo_outpost: osmo_outpost_addr,
                lp_token_denom: "uxyzabcd".into(),
                funding_duration: CwDuration::Time(0),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Default::default(),
                    asset_b_limit: Default::default(),
                },
                slippage_tolerance: None,
                pool_price_config: PoolPriceConfig {
                    expected_spot_price: Decimal::one(),
                    acceptable_price_spread: Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                },
            })
            .unwrap(),
        )
        .with_flags("--gas 1000000")
        .send()
        .unwrap();

    Ok(pooler_address)
}

/// Tests that the pooler does not advance when there is no relayer available.
/// Leaves the relayer intact after its execution.
fn test_pooler_timeout(test_ctx: &mut TestContext, pooler: &str) -> Result<(), LocalError> {
    // Stop the relayer
    test_ctx.stop_relayer();

    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    let pooler_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(pooler.to_owned()));

    // Kill the relayer and advance the pooler.
    // This should trigger SudoMsg::Timeout, which returns the state to instantiated

    // Continuously tick the pooler until the state advances to timed out (after we expect the timeout)
    with_poll_timeout! {
        {
            pooler_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_osmo_liquid_pooler::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();
        }
    }

    assert!(
        pooler_contract
            .query(
                &serde_json::to_string(
                    &valence_osmo_liquid_pooler::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data")
            == Some(&Value::String("instantiated".to_owned()))
    );

    test_ctx.start_relayer();

    Ok(())
}

/// Tests that the pooler advances after 2nd tick after a packet timeout.
fn test_pooler_timeout_recover(test_ctx: &mut TestContext, pooler: &str) -> Result<(), LocalError> {
    // Stop the relayer and ensure the pooler does not advance beyond instantiated
    test_ctx.stop_relayer();

    test_pooler_timeout(test_ctx, pooler)?;

    // Ensure that the pooler recovers after the relayer is started again
    test_ctx.start_relayer();

    test_pooler_ok(test_ctx, pooler)?;

    Ok(())
}

fn test_pooler_ok(test_ctx: &mut TestContext, pooler: &str) -> Result<(), LocalError> {
    let neutron = test_ctx.get_chain(NEUTRON_CHAIN_NAME);

    // Advance the pooler.
    // This should trigger SudoMsg::OpenAck, which will set the ContractState to IcaCreated

    // The state should be ProxyCreated
    let pooler_contract =
        CosmWasm::new_from_existing(&neutron.rb, None, None, Some(pooler.to_owned()));

    with_poll_timeout! {
        {
             pooler_contract
                .execute(
                    DEFAULT_KEY,
                    serde_json::to_string(&valence_osmo_liquid_pooler::msg::ExecuteMsg::Tick {})
                        .unwrap()
                        .as_str(),
                    "--gas 42069420",
                )
                .unwrap();

            if pooler_contract
                .query(
                    &serde_json::to_string(&valence_osmo_liquid_pooler::msg::QueryMsg::ContractState {})
                        .unwrap(),
                )
                .get("data") == Some(&Value::String("proxy_created".to_owned()))
            {
                break;
            }
        }
    }

    assert_eq!(
        pooler_contract
            .query(
                &serde_json::to_string(
                    &valence_osmo_liquid_pooler::msg::QueryMsg::ContractState {}
                )
                .unwrap()
            )
            .get("data"),
        Some(&Value::String("proxy_created".to_owned()))
    );

    Ok(())
}
